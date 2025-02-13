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
mod styles;

use app::App;
use candidate::Candidate;
use img::{
	checkers,
	is_jpeg_png,
	logo,
	with_ng_extension,
};
use refract_core::RefractError;
use styles::Skin;



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
/// Initialize and launch the GUI, or return an error.
fn main__() -> Result<(), RefractError> {
	use iced::{
		settings::Settings,
		window::{
			settings::PlatformSpecific,
			Settings as WindowSettings,
		},
	};

	let app = App::new()?;
	iced::application("Refract", App::update, App::view)
		.settings(Settings {
			default_font: Skin::FONT_REGULAR,
			default_text_size: Skin::TEXT_MD,
			..Settings::default()
		})
		.window(WindowSettings {
			// TODO: replace with `maximized: true` when stable.
			size: iced::Size::INFINITY,
			min_size: Some(iced::Size { width: 1200.0, height: 800.0 }),
			icon: img::icon(),
			platform_specific: PlatformSpecific {
				application_id: "refract".to_owned(),
				..PlatformSpecific::default()
			},
			..WindowSettings::default()
		})
		.font(include_bytes!("../skel/font/FiraMono-Bold.otf"))
		.font(include_bytes!("../skel/font/FiraMono-Regular.otf"))
		.theme(App::theme)
		.subscription(App::subscription)
		.run_with(move || {
			let task = app.start();
			(app, task)
		})
		.map_err(|_| RefractError::PrintHelp)
}
