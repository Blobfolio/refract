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
};
pub(self) use window::Window;

use atomic::Atomic;
use gio::prelude::*;
use glib::Bytes;
use gtk::prelude::*;
use refract_core::RefractError;
use std::{
	cell::RefCell,
	convert::TryFrom,
	sync::{
		Arc,
		atomic::Ordering::SeqCst,
		mpsc,
	},
};



#[macro_use]
mod macros {
	#[macro_export(local_inner_macros)]
	/// # Helper: GTK Objects From Builder.
	macro_rules! gtk_obj {
		($builder:ident, $key:literal) => (
			$builder.get_object($key).ok_or(RefractError::GtkInit)?
		);
	}

	#[macro_export(local_inner_macros)]
	/// # Helper: Toggle GTK Widget Active En Masse.
	macro_rules! gtk_active {
		($active:expr, $($obj:expr),+) => ($(
			if $obj.get_active() != $active {
				$obj.set_active($active);
			}
		)+);
	}

	#[macro_export(local_inner_macros)]
	/// # Helper: Toggle GTK Widget Sensitivity En Masse.
	macro_rules! gtk_sensitive {
		($sensitive:expr, $($obj:expr),+) => ($(
			if $obj.get_sensitive() != $sensitive {
				$obj.set_sensitive($sensitive);
			}
		)+);
	}
}



thread_local!(
	#[allow(clippy::type_complexity)] // This is the only definition.
	/// # Global.
	///
	/// This gives us a way to reach the main thread from a sister thread.
	static GLOBAL: RefCell<Option<(Arc<Window>, mpsc::Receiver<SharePayload>, Arc<Atomic<ShareFeedback>>)>> = RefCell::new(None);
);



/// # Main.
///
/// This lets us bubble up startup errors so they can be pretty-printed.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(e) => {
			eprintln!("Error: {}", e);
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
	let application =
		gtk::Application::new(Some("com.refract.gtk"), gio::ApplicationFlags::default())
			.map_err(|_| RefractError::GtkInit)?;

	application.connect_activate(|app| {
		let window = Arc::new(Window::try_from(app).expect("Unable to build GTK window."));
		setup_ui(&window);
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

/// # Setup UI.
///
/// This finishes the UI setup, hooking up communication channels, event
/// event bindings, etc.
fn setup_ui(window: &Arc<Window>) {
	let (tx, rx) = mpsc::channel();
	let fb = Arc::new(Atomic::new(ShareFeedback::Ok));
	GLOBAL.with(|global| {
		*global.borrow_mut() = Some((Arc::clone(window), rx, Arc::clone(&fb)));
	});

	// The encoder checkbox settings.
	{
		macro_rules! chk_cb {
			($validate_cb:ident, $($btn:expr),+) => {$(
				let wnd2 = Arc::clone(window);
				$btn.connect_toggled(move |_| {
					wnd2.$validate_cb();
					wnd2.paint();
				});
			)+};
		}
		chk_cb!(validate_encoders, window.chk_avif, window.chk_jxl, window.chk_webp);
		chk_cb!(validate_modes, window.chk_lossy, window.chk_lossless);
	}

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
			($btn:expr, $status:expr) => {
				let fb2 = Arc::clone(&fb);
				let wnd2 = Arc::clone(window);
				$btn.connect_clicked(move |_| {
					wnd2.remove_candidate();
					wnd2.paint();
					fb2.store($status, SeqCst);
				});
			};
		}

		feedback_cb!(window.btn_discard, ShareFeedback::Discard);
		feedback_cb!(window.btn_keep, ShareFeedback::Keep);
	}

	// The quit button.
	{
		let wnd2 = Arc::clone(window);
		window.mnu_quit.connect_activate(move |_| {
			wnd2.wnd_main.close();
		});
	}

	// Add a file!
	{
		let fb2 = Arc::clone(&fb);
		let tx2 = tx.clone();
		let wnd2 = Arc::clone(window);
		window.mnu_fopen.connect_activate(move |_| {
			if wnd2.maybe_add_file() {
				wnd2.encode(&tx2, &fb2);
			}
		});
	}

	// Add a directory!
	// Note: both tx and feedback go out of scope here.
	{
		let wnd2 = Arc::clone(window);
		window.mnu_dopen.connect_activate(move |_| {
			if wnd2.maybe_add_directory() {
				wnd2.encode(&tx, &fb);
			}
		});
	}

	// Keep the status log scrolled to the end.
	{
		let wnd2 = Arc::clone(window);
		window.lbl_status.connect_size_allocate(move |_, _| {
			if let Some(adj) = wnd2.wnd_status.get_vadjustment() {
				adj.set_value(adj.get_upper());
			}
		});
	}

	// Give it one final paint!
	window.paint();
}
