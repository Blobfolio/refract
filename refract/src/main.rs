/*!
# `Refract GTK`
*/

#![forbid(unsafe_code)]

#![warn(
	clippy::filetype_is_file,
	clippy::integer_division,
	clippy::needless_borrow,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::suboptimal_flops,
	clippy::unneeded_field_pattern,
	macro_use_extern_crate,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![allow(
	clippy::module_name_repetitions,
	clippy::redundant_pub_crate,
)]



mod candidate;
mod share;
mod window;

pub(self) use candidate::Candidate;
pub(self) use share::{
	MainTx,
	Share,
	ShareFeedback,
	SharePayload,
	SisterRx,
	SisterTx,
};
pub(self) use window::Window;

use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_VERSION,
};
use dowser::Dowser;
use gtk::{
	glib::Bytes,
	prelude::*,
};
use refract_core::RefractError;
use std::{
	path::PathBuf,
	sync::Arc,
};



pub(crate) const CLI_FORMATS: u8 =     0b0000_0111;
pub(crate) const CLI_NO_AVIF: u8 =     0b0000_0001;
pub(crate) const CLI_NO_JXL: u8 =      0b0000_0010;
pub(crate) const CLI_NO_WEBP: u8 =     0b0000_0100;

pub(crate) const CLI_MODES: u8 =       0b0001_1000;
pub(crate) const CLI_NO_LOSSLESS: u8 = 0b0000_1000;
pub(crate) const CLI_NO_LOSSY: u8 =    0b0001_0000;
pub(crate) const CLI_NO_YCBCR: u8 =    0b0010_0000;



/// # Main.
///
/// This lets us bubble up startup errors so they can be pretty-printed.
fn main() {
	match _main() {
		Ok(()) => {},
		Err(RefractError::Argue(ArgyleError::WantsVersion)) => {
			println!(concat!("Refract v", env!("CARGO_PKG_VERSION")));
		},
		Err(RefractError::Argue(ArgyleError::WantsHelp)) => {
			helper();
		},
		Err(e) => {
			eprintln!("Error: {e}");
			std::process::exit(1);
		},
	}
}

#[inline]
/// # Actual Main.
///
/// This initializes, sets up, and runs the GTK application.
///
/// ## Panics
///
/// This will panic if the building of the UI model itself fails. This
/// shouldn't ever happen, but we can't propagate that particular failure as a
/// proper `Result`.
///
/// Any other kind of issue encountered will cause the application to fail, but
/// with a pretty CLI error reason.
fn _main() -> Result<(), RefractError> {
	init_resources()?;
	let application = gtk::Application::new(
		Some("com.refract.gtk"),
		gtk::gio::ApplicationFlags::default()
	);

	// Load CLI arguments, if any.
	let args = Argue::new(FLAG_HELP | FLAG_VERSION)?.with_list();

	application.connect_activate(move |app| {
		// Parse CLI setting overrides.
		let flags = args.bitflags([
			(&b"--no-avif"[..], CLI_NO_AVIF),
			(&b"--no-jxl"[..], CLI_NO_JXL),
			(&b"--no-webp"[..], CLI_NO_WEBP),
			(&b"--no-lossless"[..], CLI_NO_LOSSLESS),
			(&b"--no-lossy"[..], CLI_NO_LOSSY),
			(&b"--no-ycbcr"[..], CLI_NO_YCBCR),
		]);

		let window = Arc::new(Window::new(app, flags).expect("Unable to build GTK window."));
		let paths = Dowser::default()
			.with_paths(args.args_os())
			.into_vec_filtered(window::is_jpeg_png);
		setup_ui(&window, paths);
		window.paint();
	});

	let args: &[&str] = &[];
	application.run_with_args(args);
	Ok(())
}

/// # Initialize Resources.
///
/// Load and register the resource bundle.
fn init_resources() -> Result<(), RefractError> {
	/// # Resource Bundle.
	const RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/resources.gresource"));
	let resources = gtk::gio::Resource::from_data(&Bytes::from(RESOURCES))
		.map_err(|_| RefractError::GtkInit)?;
	gtk::gio::resources_register(&resources);
	Ok(())
}

#[allow(clippy::similar_names)] // We're being consistent.
/// # Setup UI.
///
/// This finishes the UI setup, hooking up communication channels, event
/// bindings, etc.
fn setup_ui(window: &Arc<Window>, paths: Vec<PathBuf>) {
	let (stx, mtx, srx) = Share::init(Arc::clone(window));

	// Bind things that just need the window.
	setup_ui_window(window);

	// Discard button.
	let mtx2 = mtx.clone();
	let wnd2 = Arc::clone(window);
	window.btn_discard.connect_clicked(move |_| { wnd2.feedback(&mtx2, ShareFeedback::Discard); });

	// Keep button. (Note: mtx goes out of scope here.)
	let wnd2 = Arc::clone(window);
	window.btn_keep.connect_clicked(move |_| { wnd2.feedback(&mtx, ShareFeedback::Keep); });

	// Add a file!
	let srx2 = srx.clone();
	let stx2 = stx.clone();
	let wnd2 = Arc::clone(window);
	window.mnu_fopen.connect_activate(move |_| {
		if wnd2.maybe_add_file() { wnd2.encode(&stx2, &srx2); }
	});

	// Add file(s) via drag-and-drop.
	let wnd2 = Arc::clone(window);
	let srx2 = srx.clone();
	let stx2 = stx.clone();
	window.img_main.connect_drag_data_received(move |_, _, _, _, d, _, _| {
		for p in d.uris() {
			let file = gtk::gio::File::for_uri(&p);
			if let Some(p) = file.path() {
				wnd2.add_file(p);
			}
		}

		// Start encoding, maybe!
		wnd2.encode(&stx2, &srx2);
	});

	// Add a directory! (Note: stx and srx go out of scope here.)
	let wnd2 = Arc::clone(window);
	let srx2 = srx.clone();
	let stx2 = stx.clone();
	window.mnu_dopen.connect_activate(move |_| {
		if wnd2.maybe_add_directory() { wnd2.encode(&stx2, &srx2); }
	});

	// Add files from CLI?
	if ! paths.is_empty() {
		let mut any: bool = false;
		for path in paths {
			if window.add_file(path) {
				any = true;
			}
		}

		if any {
			window.encode(&stx, &srx);
		}
	}
}

/// # Setup UI (Callbacks Needing Window).
///
/// These event bindings require access to an `Arc<Window>`, but nothing else.
///
/// As we're using `Arc`s already, it is cheaper to clone and pass the whole
/// thing to these callbacks rather than using `glib::clone!()` to clone
/// individual references.
fn setup_ui_window(window: &Arc<Window>) {
	// The quit menu.
	let wnd2 = Arc::clone(window);
	window.mnu_quit.connect_activate(move |_| { wnd2.wnd_main.close(); });

	// The about menu.
	let wnd2 = Arc::clone(window);
	window.mnu_about.connect_activate(move |_| {
		let about = wnd2.about();
		if gtk::ResponseType::None != about.run() { about.emit_close(); }
	});

	// The A/B toggle.
	let wnd2 = Arc::clone(window);
	window.btn_toggle.connect_state_notify(move |btn| {
		wnd2.toggle_preview(btn.is_active(), true);
		wnd2.paint();
	});

	// Keep the status log scrolled to the end.
	let wnd2 = Arc::clone(window);
	window.lbl_status.connect_size_allocate(move |_, _| {
		let adj = wnd2.wnd_status.vadjustment();
		adj.set_value(adj.upper());
	});

	// Dark mode toggle.
	let wnd2 = Arc::clone(window);
	window.chk_dark.connect_toggled(move |_| { wnd2.toggle_dark(); });

	// Make sure people don't disable every encoder or encoding mode. This will
	// flip the last (just clicked) value back on if none of its sisters are
	// active.
	{
		macro_rules! chk_cb {
			($cb:ident, $($btn:ident),+) => ($(
				let wnd2 = Arc::clone(window);
				window.$btn.connect_toggled(move |btn| {
					if ! btn.is_active() && ! wnd2.$cb() { btn.set_active(true); }
				});

				// Stop the menu from closing on button press.
				window.$btn.connect_button_release_event(|btn, _| {
					btn.set_active(! btn.is_active());
					gtk::Inhibit(true)
				});
			)+);
		}

		chk_cb!(has_encoders, chk_avif, chk_jxl, chk_webp);
		chk_cb!(has_modes, chk_lossless, chk_lossy);

		// Stop the menu from closing on button press.
		window.chk_ycbcr.connect_button_release_event(|btn, _| {
			btn.set_active(! btn.is_active());
			gtk::Inhibit(true)
		});
	}

	// Sync preview field display to `lbl_quality` (so we only have to directly
	// toggle the latter).
	{
		macro_rules! preview_cb {
			($event:ident, $view:ident, $opacity:literal) => (
				let wnd2 = Arc::clone(window);
				window.lbl_quality.$event(move |_| {
					wnd2.box_ab.set_opacity($opacity);
					wnd2.lbl_format.set_opacity($opacity);
					wnd2.lbl_format_val.$view();
					wnd2.lbl_quality_val.$view();
				});
			);
		}

		preview_cb!(connect_show, show, 1.0);
		preview_cb!(connect_hide, hide, 0.0);
	}
}

#[cold]
/// # Print Help.
fn helper() {
	println!(concat!(
		r"
       ..oFaa7l;'
   =>r??\O@@@@QNk;
  :|Fjjug@@@@@@@@N}}:
 ^/aPePN@@@@peWQ@Qez;
 =iKBDB@@@O^:.::\kQO=~
 =iKQ@QWOP: ~gBQw'|Qgz,
 =i6RwEQ#s' N@RQQl i@D:   ", "\x1b[38;5;199mRefract\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r"
 =?|>a@@Nv'^Q@@@Qe ,aW|   Guided image conversion from
 ==;.\QQ@6,|Q@@@@p.;;+\,  JPEG/PNG to AVIF/JPEG-XL/WebP.
 '\tlFw9Wgs~W@@@@S   ,;'
 .^|QQp6D6t^iDRo;
   ~b@BEwDEu|:::
    rR@Q6t7|=='
     'i6Ko\=;
       `''''`

USAGE:
    refract [FLAGS] [OPTIONS] <PATH(S)>...

FORMAT FLAGS:
        --no-avif     Skip AVIF encoding.
        --no-jxl      Skip JPEG-XL encoding.
        --no-webp     Skip WebP encoding.

MODE FLAGS:
        --no-lossless Skip lossless encoding passes.
        --no-lossy    Skip lossy encoding passes.
        --no-ycbcr    Skip AVIF YCbCr encoding passes.

MISC FLAGS:
    -h, --help        Print help information and exit.
    -V, --version     Print version information and exit.

OPTIONS:
    -l, --list <FILE> Read (absolute) image and/or directory paths from this
                      text file, one path per line. This is equivalent to
                      specifying the same paths as trailing arguments, but can
                      be cleaner if there are lots of them.

TRAILING ARGS:
    <PATH(S)>...      Image and/or directory paths to re-encode. Directories
                      will be crawled recursively.
"
	));
}
