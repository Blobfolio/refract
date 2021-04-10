/*!
# `Refract`: Encoders.
*/

pub(super) mod webp;
pub(super) mod avif;

use crate::RefractError;
use ravif::{
	Img,
	RGBA8,
};
use std::num::NonZeroU8;



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Encoder.
///
/// This enum holds the conversion formats, `AVIF` and `WebP`. Source image
/// formats are instead defined by [`ImageKind`].
pub enum Encoder {
	/// # `AVIF`.
	Avif,
	/// # `WebP`.
	Webp,
}

impl Encoder {
	#[must_use]
	/// # Name.
	///
	/// Return the name of the encoder as a string slice, uppercased for
	/// consistency.
	pub const fn name(self) -> &'static str {
		match self {
			Self::Avif => "AVIF",
			Self::Webp => "WEBP",
		}
	}

	#[must_use]
	/// # File Extension.
	///
	/// Return the file extension used by the format.
	///
	/// Note: this is returned as a byte slice because that's how this program
	/// consumes it, but the values are valid UTF-8, so conversion to string,
	/// etc., can be achieved if desired.
	pub const fn ext(self) -> &'static [u8] {
		match self {
			Self::Avif => b".avif",
			Self::Webp => b".webp",
		}
	}

	#[must_use]
	/// # Error.
	///
	/// This returns the format-specific error.
	pub const fn error(self) -> RefractError {
		match self {
			Self::Avif => RefractError::NoAvif,
			Self::Webp => RefractError::NoWebp,
		}
	}

	/// # Lossy Encode.
	///
	/// Encode an image using lossy compression with the given quality setting.
	/// If successful, the new image is returned as bytes.
	///
	/// ## Errors
	///
	/// Returns an error if the image cannot be re-encoded.
	pub fn lossy(self, img: Img<&[RGBA8]>, quality: NonZeroU8)
	-> Result<Vec<u8>, RefractError> {
		match self {
			Self::Avif => avif::make_lossy(img, quality),
			Self::Webp => webp::make_lossy(img, quality),
		}
	}

	/// # Lossless Encode.
	///
	/// Encode an image using lossless compression. This only applies to
	/// [`Encoder::Webp`]; attempting the same on [`Encoder::Avif`] will always
	/// return an error.
	///
	/// ## Errors
	///
	/// Returns an error if the image cannot be re-encoded.
	pub fn lossless(self, img: Img<&[RGBA8]>) -> Result<Vec<u8>, RefractError> {
		match self {
			Self::Avif => Err(RefractError::NoAvif),
			Self::Webp => webp::make_lossless(img),
		}
	}
}
