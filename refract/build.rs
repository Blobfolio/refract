/*!
# `Refract GTK` - Build

This is used to compile a resource bundle of the various assets that need to
be pulled into GTK.
*/

use argyle::KeyWordsBuilder;
use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::PathBuf,
};



/// # Build!
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-changed=Cargo.toml");
	println!("cargo:rerun-if-changed=skel");

	build_cli();
	build_exts();
	build_imgs();
}

/// # Build CLI Keys.
fn build_cli() {
	let mut builder = KeyWordsBuilder::default();
	builder.push_keys([
		"-h", "--help",
		"--no-avif",
		"--no-jxl",
		"--no-webp",
		"--no-lossless",
		"--no-lossy",
		"--no-ycbcr",
		"-V", "--version",
	]);
	builder.push_keys_with_values(["-l", "--list"]);
	builder.save(_out_path("argyle.rs").expect("Missing OUT_DIR."));
}

/// # Build Extensions.
///
/// While we're here, we may as we pre-compute our various image extension
/// constants.
fn build_exts() {
	let out = format!(
		r"
/// # Extension: AVIF.
const E_AVIF: Extension = {};
/// # Extension: JPEG.
const E_JPEG: Extension = {};
/// # Extension: JPG.
const E_JPG: Extension = {};
/// # Extension: JXL.
const E_JXL: Extension = {};
/// # Extension: PNG.
const E_PNG: Extension = {};
/// # Extension: WEBP.
const E_WEBP: Extension = {};
",
		Extension::codegen(b"avif"),
		Extension::codegen(b"jpeg"),
		Extension::codegen(b"jpg"),
		Extension::codegen(b"jxl"),
		Extension::codegen(b"png"),
		Extension::codegen(b"webp"),
	);

	// Save them as a slice value!
	let mut file = _out_path("refract-extensions.rs")
		.and_then(|p| File::create(p).ok())
		.expect("Missing OUT_DIR.");

	file.write_all(out.as_bytes())
		.and_then(|_| file.flush())
		.expect("Unable to save extensions.");
}

/// # Build Images.
///
/// Generate checkered background images suitable for use with the light/dark
/// themes.
///
/// This shouldn't be necessary, but SVG rendering is a little wonky so it's
/// best to hardcode the background and foreground colors.
fn build_imgs() {
	use iced_core::theme::Palette;

	for (slug, palette) in [
		("Light", Palette::LIGHT),
		("Dark", Palette::DARK),
	] {
		let [br, bg, bb, _] = palette.background.into_rgba8();
		let [fr, fg, fb, _] = palette.text.into_rgba8();

		let out = format!(
			r##"<svg xmlns="http://www.w3.org/2000/svg" width="7680" height="4320" viewBox="0 0 7680 4320"><pattern id="a" width="60" height="60" x="0" y="0" patternUnits="userSpaceOnUse"><path fill="#{fr:02x}{fg:02x}{fb:02x}" fill-rule="evenodd" d="M30 30h30v30H30zM0 0h30v30H0z" opacity=".15" paint-order="fill markers"/></pattern><path fill="#{br:02x}{bg:02x}{bb:02x}" d="M0 0h7680v4320H0z"/><path fill="url(#a)" d="M0 0h7680v4320H0z"/></svg>"##,
		);

		// Save them as a slice value!
		let mut file = _out_path(&format!("refract-bg{slug}.svg"))
			.and_then(|p| File::create(p).ok())
			.expect("Missing OUT_DIR.");

		file.write_all(out.as_bytes())
			.and_then(|_| file.flush())
			.expect("Unable to save SVG.");
	}
}

/// # Output Path.
///
/// Return a path relative to the output directory.
fn _out_path(file: &str) -> Option<PathBuf> {
	let mut dir = std::fs::canonicalize(std::env::var("OUT_DIR").ok()?).ok()?;
	dir.push(file);
	Some(dir)
}
