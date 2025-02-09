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

/// # Checkered Background.
///
/// This underlay is used during image compression to help visualize
/// transparent spaces.
///
/// This only actually needs to be a tiny 60x60 pattern, but since `iced`
/// doesn't support repeating image backgrounds, we need to wrap it in an SVG
/// with a "sufficiently large" canvas.
///
/// Hopefully 8K is enough for everyone. Haha.
pub(super) static CHECKERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="7680" height="4320" viewBox="0 0 7680 4320">
	<pattern id="a" width="60" height="60" x="0" y="0" patternUnits="userSpaceOnUse">
		<path fill="#333" fill-rule="evenodd" d="M30 30h30v30H30zM0 0h30v30H0z" paint-order="fill markers"/>
		<path fill="#fff" fill-rule="evenodd" d="M0 30h30v30H0zM30 0h30v30H30z" paint-order="fill markers"/>
	</pattern>
	<path fill="url(#a)" d="M0 0h7680v4320H0z"/>
</svg>"##;

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
