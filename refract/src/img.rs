/*!
# Refract: Images
*/

use dowser::Extension;
use refract_core::ImageKind;
use std::path::{
	Path,
	PathBuf,
};



// The E_AVIF, E_JPEG, E_JPG, E_JXL, E_PNG, and E_WEBP constants are generated
// by build.rs.
include!(concat!(env!("OUT_DIR"), "/refract-extensions.rs"));

/// # Checkered Background (Light).
pub(super) static BG_LIGHT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/refract-bgLight.svg"));

/// # Checkered Background (Light).
pub(super) static BG_DARK: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/refract-bgDark.svg"));

/// # Is JPEG/PNG File.
pub(super) fn is_jpeg_png(path: &Path) -> bool {
	Extension::try_from3(path).map_or_else(
		|| Extension::try_from4(path) == Some(E_JPEG),
		|e| e == E_JPG || e == E_PNG
	)
}

/// # Fix Path Extension.
pub(super) fn with_ng_extension(mut path: PathBuf, kind: ImageKind) -> PathBuf {
	let ext = match kind {
		ImageKind::Avif =>
			if Extension::try_from4(&path) == Some(E_AVIF) { return path; }
			else { ".avif" },
		ImageKind::Jxl =>
			if Extension::try_from3(&path) == Some(E_JXL) { return path; }
			else { ".jxl" },
		ImageKind::Webp =>
			if Extension::try_from4(&path) == Some(E_WEBP) { return path; }
			else { ".webp" },
		ImageKind::Jpeg =>
			if Extension::try_from3(&path).map_or_else(
				|| Extension::try_from4(&path) == Some(E_JPEG),
				|e| e == E_JPG
			) { return path; }
			else { ".jpg" },
		ImageKind::Png =>
			if Extension::try_from3(&path) == Some(E_PNG) { return path; }
			else { ".png" },
	};

	// Append and return.
	path.as_mut_os_string().push(ext);
	path
}
