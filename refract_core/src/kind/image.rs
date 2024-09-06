/*!
# `Refract` - Image Kind
*/

use crate::{
	ImageAvif,
	ImageJpeg,
	ImageJxl,
	ImagePng,
	ImageWebp,
	Input,
	NZ_100,
	Output,
	RefractError,
	traits::DecoderResult,
};
use std::{
	fmt,
	num::NonZeroU8,
};



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Image Kind.
pub enum ImageKind {
	/// # AVIF.
	Avif,

	/// # JPEG.
	Jpeg,

	/// # JPEG XL.
	Jxl,

	/// # PNG.
	Png,

	/// # WebP.
	Webp,
}

impl AsRef<str> for ImageKind {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for ImageKind {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl TryFrom<&[u8]> for ImageKind {
	type Error = RefractError;

	/// # From Raw Bytes.
	///
	/// This examines the first 12 bytes of the raw image file to see what
	/// magic its headers contain.
	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		// We need at least twelve bytes to hold header info!
		if src.len() > 12 {
			// PNG has just one way to be!
			if src[..8] == [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'] {
				return Ok(Self::Png);
			}

			// WebP is fairly straightforward.
			if src[..4] == *b"RIFF" && src[8..12] == *b"WEBP" {
				return Ok(Self::Webp);
			}

			// AVIF has a few ways to be. We're ignoring sequences since we
			// aren't building them.
			if
				src[4..8] == *b"ftyp" &&
				matches!(&src[8..12], b"avif" | b"MA1B" | b"MA1A")
			{
				return Ok(Self::Avif);
			}

			// JPEG XL can either be a codestream or containerized.
			if
				src[..2] == [0xFF, 0x0A] ||
				src[..12] == [0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', 0x20, 0x0D, 0x0A, 0x87, 0x0A]
			{
				return Ok(Self::Jxl);
			}

			// JPEG can look a few different ways, particularly in the middle.
			if
				src[..3] == [0xFF, 0xD8, 0xFF] &&
				src[src.len() - 2..] == [0xFF, 0xD9] &&
				(
					src[3] == 0xDB ||
					src[3] == 0xEE ||
					(src[3..12] == [0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01]) ||
					(src[3] == 0xE1 && src[6..12] == [b'E', b'x', b'i', b'f', 0x00, 0x00])
				)
			{
				return Ok(Self::Jpeg);
			}
		}

		Err(RefractError::Image)
	}
}

/// ## Information.
impl ImageKind {
	#[cfg(not(feature = "decode_ng"))]
	#[inline]
	#[must_use]
	/// # Can Decode?
	///
	/// Returns `true` if decoding is supported for this image type.
	///
	/// When the feature flag `decode_ng` is used, this always returns `true`.
	pub const fn can_decode(self) -> bool { matches!(self, Self::Jpeg | Self::Png) }

	#[cfg(feature = "decode_ng")]
	#[inline]
	#[must_use]
	/// # Can Decode?
	///
	/// Returns `true` if decoding is supported for this image type.
	///
	/// When the feature flag `decode_ng` is used, this always returns `true`.
	pub const fn can_decode(self) -> bool { true }

	#[inline]
	#[must_use]
	/// # Can Encode?
	///
	/// Returns `true` if encoding is supported for this image type.
	pub const fn can_encode(self) -> bool {
		matches!(self, Self::Avif | Self::Jxl | Self::Webp)
	}
}

/// ## Getters.
impl ImageKind {
	#[must_use]
	/// # As String Slice.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Avif => "AVIF",
			Self::Jpeg => "JPEG",
			Self::Jxl => "JPEG XL",
			Self::Png => "PNG",
			Self::Webp => "WebP",
		}
	}

	#[must_use]
	/// # File Extension.
	pub const fn extension(self) -> &'static str {
		match self {
			Self::Avif => "avif",
			Self::Jpeg => "jpg",
			Self::Jxl => "jxl",
			Self::Png => "png",
			Self::Webp => "webp",
		}
	}

	#[must_use]
	/// # Media Type.
	pub const fn mime(self) -> &'static str {
		match self {
			Self::Avif => "image/avif",
			Self::Jpeg => "image/jpeg",
			Self::Jxl => "image/jxl",
			Self::Png => "image/png",
			Self::Webp => "image/webp",
		}
	}

	#[expect(clippy::unused_self, reason = "We may need `self` in the future.")]
	#[must_use]
	/// # Encoding Minimum Quality.
	///
	/// At the moment, this always returns `1`.
	pub(crate) const fn min_encoder_quality(self) -> NonZeroU8 { NonZeroU8::MIN }

	#[must_use]
	/// # Encoding Minimum Quality.
	///
	/// This returns the maximum encoding quality value for the given format,
	/// or a default of `100`.
	pub(crate) const fn max_encoder_quality(self) -> NonZeroU8 {
		use crate::traits::Encoder;

		match self {
			Self::Avif => ImageAvif::MAX_QUALITY,
			Self::Jxl => ImageJxl::MAX_QUALITY,
			_ => NZ_100,
		}
	}
}

/// ## Decoding.
impl ImageKind {
	/// # Decode.
	///
	/// Decode a raw image of this kind into RGBA pixels (and width, height,
	/// and color type).
	///
	/// Decoding support for the next-gen formats can be enabled with the
	/// feature flag `decode_ng`. Otherwise only JPEG and PNG image sources
	/// can be decoded.
	///
	/// ## Errors
	///
	/// This will bubble up any decoder errors encountered, including cases
	/// where decoding is unsupported for the format.
	pub fn decode(self, raw: &[u8]) -> Result<DecoderResult, RefractError> {
		use crate::traits::Decoder;

		match self {
			Self::Jpeg => ImageJpeg::decode(raw),
			Self::Png => ImagePng::decode(raw),

			#[cfg(feature = "decode_ng")] Self::Avif => ImageAvif::decode(raw),
			#[cfg(feature = "decode_ng")] Self::Jxl => ImageJxl::decode(raw),
			#[cfg(feature = "decode_ng")] Self::Webp => ImageWebp::decode(raw),
			#[cfg(not(feature = "decode_ng"))] _ => Err(RefractError::ImageDecode(self)),
		}
	}
}

/// ## Encoding.
impl ImageKind {
	/// # Encode Lossy.
	///
	/// Encode pixels into a raw image using lossy compression.
	///
	/// ## Errors
	///
	/// This will bubble up any encoder errors encountered, including cases
	/// where encoding is unsupported for the format.
	pub fn encode_lossy(
		self,
		input: &Input,
		output: &mut Output,
		quality: NonZeroU8,
		flags: u8
	) -> Result<(), RefractError> {
		use crate::traits::Encoder;

		match self {
			Self::Avif => ImageAvif::encode_lossy(input, output, quality, flags),
			Self::Jxl => ImageJxl::encode_lossy(input, output, quality, flags),
			Self::Webp => ImageWebp::encode_lossy(input, output, quality, flags),
			_ => Err(RefractError::ImageEncode(self)),
		}
	}

	/// # Encode Lossless.
	///
	/// Encode pixels into a raw image using lossless compression.
	///
	/// ## Errors
	///
	/// This will bubble up any encoder errors encountered, including cases
	/// where encoding is unsupported for the format.
	pub fn encode_lossless(
		self,
		input: &Input,
		output: &mut Output,
		flags: u8
	) -> Result<(), RefractError> {
		use crate::traits::Encoder;

		match self {
			Self::Avif => ImageAvif::encode_lossless(input, output, flags),
			Self::Jxl => ImageJxl::encode_lossless(input, output, flags),
			Self::Webp => ImageWebp::encode_lossless(input, output, flags),
			_ => Err(RefractError::ImageEncode(self)),
		}
	}
}
