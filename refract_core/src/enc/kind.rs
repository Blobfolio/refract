/*!
# `Refract` - Encoders
*/

/*!
# `Refract` - Image Kind
*/

use crate::RefractError;
use std::{
	convert::TryFrom,
	fmt,
	num::NonZeroU8,
};



const MIN_QUALITY: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(1) };



#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Image Kind.
///
/// A list of supported image kinds.
pub enum OutputKind {
	/// # `AVIF`.
	Avif,
	/// # `JPEG XL`.
	Jxl,
	/// # `WebP`.
	Webp,
}

impl fmt::Display for OutputKind {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(unsafe { std::str::from_utf8_unchecked(self.as_bytes()) })
	}
}

impl TryFrom<&[u8]> for OutputKind {
	type Error = RefractError;

	/// # From Bytes.
	///
	/// Obtain the image kind from the raw file bytes by inspecting its magic
	/// headers.
	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		// If the source is big enough for headers, keep going!
		if src.len() > 12 {
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
		}

		Err(RefractError::Output)
	}
}

/// ## Byte Getters.
impl OutputKind {
	#[must_use]
	/// # Extension (bytes).
	///
	/// Return the extension, including the leading period, as a byte slice.
	pub const fn ext_bytes(self) -> &'static [u8] {
		match self {
			Self::Avif => b".avif",
			Self::Jxl => b".jxl",
			Self::Webp => b".webp",
		}
	}

	#[must_use]
	/// # Name (bytes).
	///
	/// Return the formatted name of the type as a byte slice.
	pub const fn as_bytes(self) -> &'static [u8] {
		match self {
			Self::Avif => b"AVIF",
			Self::Jxl => b"JPEG XL",
			Self::Webp => b"WebP",
		}
	}
}

/// ## Getters.
impl OutputKind {
	#[must_use]
	/// # Quality Range.
	pub const fn quality_range(self) -> (NonZeroU8, NonZeroU8) {
		match self {
			Self::Avif => (MIN_QUALITY, unsafe { NonZeroU8::new_unchecked(63) }),
			Self::Jxl => (MIN_QUALITY, unsafe { NonZeroU8::new_unchecked(150) }),
			Self::Webp => (MIN_QUALITY, unsafe { NonZeroU8::new_unchecked(100) }),
		}
	}

	#[must_use]
	/// # Lossless Quality.
	pub const fn lossless_quality(self) -> NonZeroU8 {
		match self {
			Self::Avif => unsafe { NonZeroU8::new_unchecked(255) },
			Self::Jxl => unsafe { NonZeroU8::new_unchecked(150) },
			Self::Webp => unsafe { NonZeroU8::new_unchecked(100) },
		}
	}
}
