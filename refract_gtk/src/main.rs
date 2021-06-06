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
/// This finishes the UI setup, hooking up communication channels, and event
/// bindings requiring those channels (or the [`Window`] as a whole).
fn setup_ui(window: &Arc<Window>) {
	let (stx, mtx, srx) = Share::init(Arc::clone(window));

	// The toggle.
	{
		let wnd2 = Arc::clone(window);
		window.btn_toggle.connect_property_state_notify(move |btn| {
			wnd2.toggle_preview(btn.get_active(), true);
			wnd2.paint();
		});
	}

	// Discard/Keep button.
	{
		macro_rules! feedback_cb {
			($(($btn:ident, $status:ident)),+) => ($(
				let mtx2 = mtx.clone();
				let wnd2 = Arc::clone(window);
				window.$btn.connect_clicked(move |_| {
					wnd2.remove_candidate();
					wnd2.paint();
					mtx2.send(ShareFeedback::$status).unwrap();
				});
			)+);
		}

		feedback_cb!((btn_discard, Discard), (btn_keep, Keep));
	}

	// Add a file!
	{
		let srx2 = srx.clone();
		let stx2 = stx.clone();
		let wnd2 = Arc::clone(window);
		window.mnu_fopen.connect_activate(move |_| {
			if wnd2.maybe_add_file() { wnd2.encode(&stx2, &srx2); }
		});
	}

	// Add a directory!
	// Note: both stx and srx go out of scope here.
	{
		let wnd2 = Arc::clone(window);
		window.mnu_dopen.connect_activate(move |_| {
			if wnd2.maybe_add_directory() { wnd2.encode(&stx, &srx); }
		});
	}
}
