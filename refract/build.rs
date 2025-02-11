/*!
# Refract - Build
*/

use argyle::KeyWordsBuilder;
use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
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
		"-s", "--save-auto",
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
/// Pre-decode a couple images for embed to lessen the runtime costs of using
/// them.
fn build_imgs() {
	/// # Load PNG.
	///
	/// Being a build script, this will just panic with an appropriate message
	/// if something goes awry.
	fn load_png<P: AsRef<Path>>(src: P) -> (usize, Vec<u8>) {
		let src: &Path = src.as_ref();
		let Ok(raw) = std::fs::read(src) else { panic!("Missing {}", src.display()); };
		let Ok(dec) = refract_core::Input::try_from(raw.as_slice()) else {
			panic!("Unable to decode {}", src.display());
		};

		let width = dec.width();
		assert_eq!(width, dec.height(), "Non-square image {}", src.display());

		(width, dec.into_rgba().take_pixels())
	}

	let (icon_size, icon) = load_png("skel/deb/icons/hicolor/128x128/apps/refract.png");
	let (logo_size, logo) = load_png("skel/img/logo.png");

	let (tmp, checkers0) = load_png("skel/img/checkers0.png");
	assert_eq!(tmp, 60, "Bug: checker tiles must be 60x60."); // Sanity check.
	let (tmp, checkers1) = load_png("skel/img/checkers1.png");
	assert_eq!(tmp, 60, "Bug: checker tiles must be 60x60."); // Sanity check.

	let out = format!(
		"/// # Checkers.
pub(super) fn checkers() -> (image::Handle, image::Handle) {{
	(
		tile_checkers(&{checkers0:?}),
		tile_checkers(&{checkers1:?}),
	)
}}

/// # Program Icon.
pub(super) fn icon() -> Option<Icon> {{
	icon::from_rgba(vec!{icon:?}, {icon_size}, {icon_size}).ok()
}}

/// # Logo.
pub(super) fn logo() -> image::Handle {{
	static LOGO: &[u8] = &{logo:?};
	image::Handle::from_rgba({logo_size}, {logo_size}, LOGO)
}}
",
	);

	// Save it!
	let mut file = _out_path("refract-img.rs")
		.and_then(|p| File::create(p).ok())
		.expect("Missing OUT_DIR.");

	file.write_all(out.as_bytes())
		.and_then(|_| file.flush())
		.expect("Unable to save img.");
}

/// # Output Path.
///
/// Return a path relative to the output directory.
fn _out_path(file: &str) -> Option<PathBuf> {
	let mut dir = std::fs::canonicalize(std::env::var("OUT_DIR").ok()?).ok()?;
	dir.push(file);
	Some(dir)
}
