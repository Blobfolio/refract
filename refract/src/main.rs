/*!
# Refract
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



mod app;
mod candidate;
mod img;

use app::App;
use candidate::Candidate;
use img::{
	BG_DARK,
	BG_LIGHT,
	is_jpeg_png,
	with_ng_extension,
};
use refract_core::RefractError;



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
	use iced::{
		font::Font,
		settings::Settings,
		window::{
			settings::PlatformSpecific,
			Settings as WindowSettings,
		},
	};

	let app = App::new()?;
	iced::application("Refract", App::update, App::view)
		.settings(Settings {
			default_font: Font::MONOSPACE,
			default_text_size: 14_u16.into(),
			..Settings::default()
		})
		.window(WindowSettings {
			size: iced::Size::INFINITY,
			platform_specific: PlatformSpecific {
				application_id: "refract".to_owned(),
				..PlatformSpecific::default()
			},
			..WindowSettings::default()
		})
		.theme(App::theme)
		.subscription(App::subscription)
		.run_with(move || {
			let task = app.start();
			(app, task)
		})
		.map_err(|_| RefractError::PrintHelp)
}
