/*!
# `Refract` - Traits.
*/

use crate::{
	ColorKind,
	RefractError,
};

#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
use std::num::NonZeroU8;

#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
use crate::{
	Input,
	NZ_100,
	Output,
};



/// # The result type for `Decoder::decode`.
pub(super) type DecoderResult = (Vec<u8>, usize, usize, ColorKind);

/// # Decoder.
///
/// This is implemented for image formats capable of decoding raw image data
/// into RGBA pixels.
pub(super) trait Decoder {
	/// # Decode.
	///
	/// Decode the bytes from a raw image file into a contiguous `u8` buffer
	/// using 4 bytes (RGBA) per pixel.
	///
	/// RGB, greyscale, etc., should be upscaled accordingly.
	///
	/// ## Errors
	///
	/// Return any errors encountered during decoding.
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError>;
}

#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
/// # Encoder.
///
/// This is implemented for image formats capable of encoding from RGBA pixels
/// into a raw image.
pub(super) trait Encoder {
	/// # Minimum Quality.
	const MIN_QUALITY: NonZeroU8 = NonZeroU8::MIN;

	/// # Maximum Quality.
	const MAX_QUALITY: NonZeroU8 = NZ_100;

	/// # Encode Lossy.
	///
	/// Encode a slice of pixels into a complete image using lossy compression
	/// at the specified quality.
	///
	/// ## Errors
	///
	/// Return any errors encountered during decoding.
	fn encode_lossy(input: &Input, output: &mut Output, quality: NonZeroU8, flags: u8)
	-> Result<(), RefractError>;

	/// # Encode Lossless.
	///
	/// Encode a slice of pixels into a complete image using lossless
	/// compression.
	///
	/// ## Errors
	///
	/// Return any errors encountered during decoding.
	fn encode_lossless(input: &Input, output: &mut Output, flags: u8)
	-> Result<(), RefractError>;
}
