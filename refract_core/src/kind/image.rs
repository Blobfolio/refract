/*!
# `Refract` - Image Kind
*/

#![allow(unreachable_patterns, reason = "The feature combinations make this impractical.")]

use crate::RefractError;

#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
use crate::{
	Input,
	NZ_100,
	Output,
};

use crate::traits::DecoderResult;

#[cfg(feature = "avif")] use crate::ImageAvif;
#[cfg(feature = "jpeg")] use crate::ImageJpeg;
#[cfg(feature = "jxl")]  use crate::ImageJxl;
#[cfg(feature = "png")]  use crate::ImagePng;
#[cfg(feature = "webp")] use crate::ImageWebp;

use std::fmt;

#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
use std::num::NonZeroU8;



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

	/// # Invalid.
	Invalid,
}

impl AsRef<str> for ImageKind {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for ImageKind {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.pad(self.as_str()) }
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
	#[inline]
	#[must_use]
	/// # Can Decode?
	///
	/// Returns `true` if decoding is supported for this image type.
	///
	/// When the feature flag `decode_ng` is used, this always returns `true`.
	pub const fn can_decode(self) -> bool {
		match self {
			#[cfg(feature = "avif")] Self::Avif => true,
			#[cfg(feature = "jpeg")] Self::Jpeg => true,
			#[cfg(feature = "jxl")]  Self::Jxl => true,
			#[cfg(feature = "png")]  Self::Png => true,
			#[cfg(feature = "webp")] Self::Webp => true,
			_ => false,
		}
	}

	#[inline]
	#[must_use]
	/// # Can Encode?
	///
	/// Returns `true` if encoding is supported for this image type.
	pub const fn can_encode(self) -> bool {
		match self {
			#[cfg(feature = "avif")] Self::Avif => true,
			#[cfg(feature = "jxl")]  Self::Jxl => true,
			#[cfg(feature = "webp")] Self::Webp => true,
			_ => false,
		}
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
			Self::Invalid => "???",
		}
	}

	#[must_use]
	/// # Is Empty?
	///
	/// Added only for consistency; image kinds are never empty.
	pub const fn is_empty(self) -> bool { false }

	#[must_use]
	/// # Length.
	pub const fn len(self) -> usize {
		match self {
			Self::Avif | Self::Jpeg | Self::Webp => 4,
			Self::Jxl => 7,
			Self::Png | Self::Invalid => 3,
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
			Self::Invalid => "xxx",
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
			Self::Invalid => "application/octet-stream",
		}
	}

	#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
	#[expect(clippy::unused_self, reason = "We may need `self` in the future.")]
	#[must_use]
	/// # Encoding Minimum Quality.
	///
	/// At the moment, this always returns `1`.
	pub(crate) const fn min_encoder_quality(self) -> NonZeroU8 { NonZeroU8::MIN }

	#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
	#[must_use]
	/// # Encoding Minimum Quality.
	///
	/// This returns the maximum encoding quality value for the given format,
	/// or a default of `100`.
	pub(crate) const fn max_encoder_quality(self) -> NonZeroU8 {
		#[cfg(any(feature = "avif", feature = "jxl"))] use crate::traits::Encoder;

		match self {
			#[cfg(feature = "avif")] Self::Avif => ImageAvif::MAX_QUALITY,
			#[cfg(feature = "jxl")]  Self::Jxl => ImageJxl::MAX_QUALITY,
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
	/// ## Errors
	///
	/// This will bubble up any decoder errors encountered, including cases
	/// where decoding is unsupported for the format.
	pub fn decode(self, raw: &[u8]) -> Result<DecoderResult, RefractError> {
		use crate::traits::Decoder;

		match self {
			#[cfg(feature = "jpeg")] Self::Jpeg => ImageJpeg::decode(raw),
			#[cfg(feature = "png")]  Self::Png => ImagePng::decode(raw),
			#[cfg(feature = "avif")] Self::Avif => ImageAvif::decode(raw),
			#[cfg(feature = "jxl")]  Self::Jxl => ImageJxl::decode(raw),
			#[cfg(feature = "webp")] Self::Webp => ImageWebp::decode(raw),

			_ => Err(RefractError::ImageDecode(self)),
		}
	}
}

#[cfg(any(feature = "avif", feature = "jxl", feature = "webp"))]
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
			#[cfg(feature = "avif")] Self::Avif => ImageAvif::encode_lossy(input, output, quality, flags),
			#[cfg(feature = "jxl")]  Self::Jxl => ImageJxl::encode_lossy(input, output, quality, flags),
			#[cfg(feature = "webp")] Self::Webp => ImageWebp::encode_lossy(input, output, quality, flags),
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
			#[cfg(feature = "avif")] Self::Avif => ImageAvif::encode_lossless(input, output, flags),
			#[cfg(feature = "jxl")]  Self::Jxl => ImageJxl::encode_lossless(input, output, flags),
			#[cfg(feature = "webp")] Self::Webp => ImageWebp::encode_lossless(input, output, flags),
			_ => Err(RefractError::ImageEncode(self)),
		}
	}
}
