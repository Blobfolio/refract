/*!
# `Refract`: Error
*/

use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Error.
pub(super) enum RefractError {
	/// # Invalid Image.
	InvalidImage,
	/// # Unable to produce an acceptable AVIF version.
	NoAvif,
	/// # Unable to produce an acceptable WebP version.
	NoWebp,
	/// # Write Fail.
	Write,
}

impl Error for RefractError {}

impl fmt::Display for RefractError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl RefractError {
	#[must_use]
	/// # As Str.
	pub(crate) const fn as_str(self) -> &'static str {
		match self {
			Self::InvalidImage => "The image is invalid or unreadable.",
			Self::NoAvif => "No acceptable AVIF candidate was found.",
			Self::NoWebp => "No acceptable WebP candidate was found.",
			Self::Write => "Unable to save the image.",
		}
	}
}
