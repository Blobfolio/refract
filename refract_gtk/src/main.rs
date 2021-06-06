/*!
# `Refract GTK`
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::module_name_repetitions)]



mod candidate;
pub(self) mod l10n;
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

use gio::prelude::*;
use glib::Bytes;
use gtk::prelude::*;
use refract_core::RefractError;
use std::{
	convert::TryFrom,
	sync::Arc,
};



/// # Main.
///
/// This lets us bubble up startup errors so they can be pretty-printed.
fn main() {
	if let Err(e) = _main() {
		eprintln!("Error: {}", e);
		std::process::exit(1);
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
	let application =
		gtk::Application::new(Some("com.refract.gtk"), gio::ApplicationFlags::default())
			.map_err(|_| RefractError::GtkInit)?;

	application.connect_activate(|app| {
		let window = Arc::new(Window::try_from(app).expect("Unable to build GTK window."));
		setup_ui(&window);
		window.paint();
	});

	application.run(&[]);
	Ok(())
}

/// # Initialize Resources.
///
/// Load and register the resource bundle.
fn init_resources() -> Result<(), RefractError> {
	/// # Resource Bundle.
	const RESOURCES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/resources.gresource"));
	let resources = gio::Resource::from_data(&Bytes::from(RESOURCES))
		.map_err(|_| RefractError::GtkInit)?;
	gio::resources_register(&resources);
	Ok(())
}

#[allow(clippy::similar_names)] // We're being consistent.
/// # Setup UI.
///
/// This finishes the UI setup, hooking up communication channels, event
/// bindings, etc.
fn setup_ui(window: &Arc<Window>) {
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

	// Add a directory! (Note: stx and srx go out of scope here.)
	let wnd2 = Arc::clone(window);
	window.mnu_dopen.connect_activate(move |_| {
		if wnd2.maybe_add_directory() { wnd2.encode(&stx, &srx); }
	});
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
	window.btn_toggle.connect_property_state_notify(move |btn| {
		wnd2.toggle_preview(btn.get_active(), true);
		wnd2.paint();
	});

	// Keep the status log scrolled to the end.
	let wnd2 = Arc::clone(window);
	window.lbl_status.connect_size_allocate(move |_, _| {
		if let Some(adj) = wnd2.wnd_status.get_vadjustment() {
			adj.set_value(adj.get_upper());
		}
	});

	// Make sure people don't disable every encoder or encoding mode. This will
	// flip the last (just clicked) value back on if none of its sisters are
	// active.
	{
		macro_rules! chk_cb {
			($cb:ident, $($btn:ident),+) => ($(
				let wnd2 = Arc::clone(window);
				window.$btn.connect_toggled(move |btn| {
					if ! btn.get_active() && ! wnd2.$cb() { btn.set_active(true); }
				});
			)+);
		}

		chk_cb!(has_encoders, chk_avif, chk_jxl, chk_webp);
		chk_cb!(has_modes, chk_lossless, chk_lossy);
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
