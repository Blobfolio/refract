/*!
# `Refract`: Error
*/

use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Error.
pub enum RefractError {
	/// # Invalid Image.
	InvalidImage,
	/// # Unable to produce an acceptable AVIF version.
	NoAvif,
	/// # Unable to produce an acceptable WebP version.
	NoWebp,
	/// # Too Big.
	TooBig,
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
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::InvalidImage => "The image is not a valid JPEG or PNG.",
			Self::NoAvif => "No acceptable AVIF was found.",
			Self::NoWebp => "No acceptable WebP was found.",
			Self::TooBig => "The converted image was larger than the source.",
			Self::Write => "Unable to save the image.",
		}
	}
}
