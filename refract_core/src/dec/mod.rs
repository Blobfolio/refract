/*!
# `Refract` - Decoding!
*/

mod alpha;
mod jpeg;
mod png;



use crate::{
	ColorKind,
	ImageKind,
	RefractError,
};
use std::convert::TryFrom;



pub(self) type RawDecoded = (Vec<u8>, usize, usize, ColorKind);



/// # Decode.
///
/// Decode a raw file source into delicious pixels.
///
/// This will always return a contiguous `u8` buffer using 4 bytes per pixel.
/// Greyscale, RGB, etc., will be upscaled accordingly.
///
/// ## Errors
///
/// This will return an error if the image is invalid or cannot be decoded.
pub fn decode(raw: &[u8])
-> Result<(Vec<u8>, usize, usize, ColorKind, ImageKind), RefractError> {
	let kind = ImageKind::try_from(raw)?;
	let (buf, width, height, color) = match kind {
		ImageKind::Jpeg => jpeg::decode(raw)?,
		ImageKind::Png => png::decode(raw)?,
		// Decoding other image types is not (yet) supported. This will likely
		// be added behind a feature flag in a future release.
		_ => return Err(RefractError::ImageDecode(kind)),
	};

	Ok((buf, width, height, color, kind))
}
