/*!
# `Refract GTK`
*/

#![forbid(unsafe_code)]

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]



mod candidate;
mod share;
mod window;

use candidate::Candidate;
use share::{
	MainTx,
	Share,
	ShareFeedback,
	SharePayload,
	SisterRx,
	SisterTx,
};
use window::Window;

use argyle::Argument;
use dowser::Dowser;
use gtk::{
	glib::Bytes,
	prelude::*,
};
use refract_core::RefractError;
use std::{
	path::PathBuf,
	rc::Rc,
};



/// # CLI Flag: Format Bits.
pub(crate) const CLI_FORMATS: u8 =     0b0000_0111;

/// # CLI Flag: No Avif.
pub(crate) const CLI_NO_AVIF: u8 =     0b0000_0001;

/// # CLI Flag: No JXL.
pub(crate) const CLI_NO_JXL: u8 =      0b0000_0010;

/// # CLI Flag: No WebP
pub(crate) const CLI_NO_WEBP: u8 =     0b0000_0100;

/// # CLI Flag: Mode Bits.
pub(crate) const CLI_MODES: u8 =       0b0001_1000;

/// # CLI Flag: No Lossless.
pub(crate) const CLI_NO_LOSSLESS: u8 = 0b0000_1000;

/// # CLI Flag: No Lossy.
pub(crate) const CLI_NO_LOSSY: u8 =    0b0001_0000;

/// # CLI Flag: No Ycbcr.
pub(crate) const CLI_NO_YCBCR: u8 =    0b0010_0000;



/// # Main.
///
/// This lets us bubble up startup errors so they can be pretty-printed.
fn main() {
	match main__() {
		Ok(()) => {},
		Err(e @ (RefractError::PrintHelp | RefractError::PrintVersion)) => {
			println!("{e}");
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
fn main__() -> Result<(), RefractError> {
	init_resources()?;
	let application = gtk::Application::new(
		Some("com.refract.gtk"),
		gtk::gio::ApplicationFlags::default()
	);

	// Load CLI arguments, if any.
	let args = argyle::args()
		.with_keywords(include!(concat!(env!("OUT_DIR"), "/argyle.rs")));

	let mut paths = Dowser::default();
	let mut flags = 0_u8;
	for arg in args {
		match arg {
			Argument::Key("-h" | "--help") => return Err(RefractError::PrintHelp),
			Argument::Key("--no-avif") => { flags |= CLI_NO_AVIF; },
			Argument::Key("--no-jxl") => { flags |= CLI_NO_JXL; },
			Argument::Key("--no-webp") => { flags |= CLI_NO_WEBP; },
			Argument::Key("--no-lossless") => { flags |= CLI_NO_LOSSLESS; },
			Argument::Key("--no-lossy") => { flags |= CLI_NO_LOSSY; },
			Argument::Key("--no-ycbcr") => { flags |= CLI_NO_YCBCR; },
			Argument::Key("-V" | "--version") => return Err(RefractError::PrintVersion),

			Argument::KeyWithValue("-l" | "--list", s) => {
				let _res = paths.read_paths_from_file(s);
			},

			// Assume paths.
			Argument::Other(s) => { paths = paths.with_path(s); },
			Argument::InvalidUtf8(s) => { paths = paths.with_path(s); },

			// Nothing else is relevant.
			_ => {},
		}
	}

	application.connect_activate(move |app| {
		let window = Rc::new(Window::new(app, flags)
				.expect("Unable to build GTK window."));

		// We have to clone this because GTK doesn't do Rust properly. Haha.
		let paths = paths.clone().into_vec_filtered(window::is_jpeg_png);

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

#[expect(clippy::similar_names, reason = "Consistency wins here.")]
/// # Setup UI.
///
/// This finishes the UI setup, hooking up communication channels, event
/// bindings, etc.
fn setup_ui(window: &Rc<Window>, paths: Vec<PathBuf>) {
	let (stx, mtx, srx) = Share::init(Rc::clone(window));

	// Bind things that just need the window.
	setup_ui_window(window);

	// Discard button.
	let mtx2 = mtx.clone();
	let wnd2 = Rc::clone(window);
	window.btn_discard.connect_clicked(move |_| { wnd2.feedback(&mtx2, ShareFeedback::Discard); });

	// Keep button. (Note: mtx goes out of scope here.)
	let wnd2 = Rc::clone(window);
	window.btn_keep.connect_clicked(move |_| { wnd2.feedback(&mtx, ShareFeedback::Keep); });

	// Add a file!
	let srx2 = srx.clone();
	let stx2 = stx.clone();
	let wnd2 = Rc::clone(window);
	window.mnu_fopen.connect_activate(move |_| {
		if wnd2.maybe_add_file() { wnd2.encode(&stx2, &srx2); }
	});

	// Add file(s) via drag-and-drop.
	let wnd2 = Rc::clone(window);
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
	let wnd2 = Rc::clone(window);
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
fn setup_ui_window(window: &Rc<Window>) {
	// The quit menu.
	let wnd2 = Rc::clone(window);
	window.mnu_quit.connect_activate(move |_| { wnd2.wnd_main.close(); });

	// The about menu.
	let wnd2 = Rc::clone(window);
	window.mnu_about.connect_activate(move |_| {
		let about = wnd2.about();
		if gtk::ResponseType::None != about.run() { about.emit_close(); }
	});

	// The A/B toggle.
	let wnd2 = Rc::clone(window);
	window.btn_toggle.connect_state_notify(move |btn| {
		wnd2.toggle_preview(btn.is_active(), true);
		wnd2.paint();
	});

	// Keep the status log scrolled to the end.
	let wnd2 = Rc::clone(window);
	window.lbl_status.connect_size_allocate(move |_, _| {
		let adj = wnd2.wnd_status.vadjustment();
		adj.set_value(adj.upper());
	});

	// Dark mode toggle.
	let wnd2 = Rc::clone(window);
	window.chk_dark.connect_toggled(move |_| { wnd2.toggle_dark(); });

	// Make sure people don't disable every encoder or encoding mode. This will
	// flip the last (just clicked) value back on if none of its sisters are
	// active.
	{
		/// # Helper Check Handling.
		macro_rules! chk_cb {
			($cb:ident, $($btn:ident),+) => ($(
				let wnd2 = Rc::clone(window);
				window.$btn.connect_toggled(move |btn| {
					if ! btn.is_active() && ! wnd2.$cb() { btn.set_active(true); }
				});

				// Stop the menu from closing on button press.
				window.$btn.connect_button_release_event(|btn, _| {
					btn.set_active(! btn.is_active());
					gtk::glib::Propagation::Stop
				});
			)+);
		}

		chk_cb!(has_encoders, chk_avif, chk_jxl, chk_webp);
		chk_cb!(has_modes, chk_lossless, chk_lossy);

		// Stop the menu from closing on button press.
		window.chk_ycbcr.connect_button_release_event(|btn, _| {
			btn.set_active(! btn.is_active());
			gtk::glib::Propagation::Stop
		});
	}

	// Sync preview field display to `lbl_quality` (so we only have to directly
	// toggle the latter).
	{
		/// # Helper: Preview Handling.
		macro_rules! preview_cb {
			($event:ident, $view:ident, $opacity:literal) => (
				let wnd2 = Rc::clone(window);
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
