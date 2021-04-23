/*!
# `Refract` - Error
*/

use crate::OutputKind;
#[cfg(feature = "menu")] use argyle::ArgyleError;
use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Error.
pub enum RefractError {
	/// # No candidate found.
	Candidate(OutputKind),
	/// # Encoding error.
	Encode,
	/// # No Encoders.
	NoEncoders,
	/// # No Images.
	NoImages,
	/// # Encoder does not support lossless mode.
	NoLossless,
	/// # Unable to read source.
	Read,
	/// # Invalid image source.
	Source,
	/// # Candidate too big.
	TooBig,
	/// # Unable to write to file.
	Write,

	#[cfg(feature = "menu")]
	/// # Passthrough menu error.
	Menu(ArgyleError),
}

impl Error for RefractError {}

impl AsRef<str> for RefractError {
	fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for RefractError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl RefractError {
	#[must_use]
	/// # As Str.
	///
	/// Return the error as an English string slice.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Candidate(kind) => match kind {
				OutputKind::Avif => "No acceptable AVIF candidate was found.",
				#[cfg(feature = "jxl")] OutputKind::Jxl => "No acceptable JPEG XL candidate was found.",
				OutputKind::Webp => "No acceptable WebP candidate was found.",
			},
			Self::Encode => "Errors were encountered while trying to encode the image.",
			Self::NoEncoders => "You've disabled all encoders; there is nothing to do!",
			Self::NoImages => "No images were found.",
			Self::NoLossless => "Lossless encoding is not supported.",
			Self::Read => "Unable to read the source image.",
			Self::Source => "Invalid image source.",
			Self::TooBig => "The encoded image was larger than the source.",
			Self::Write => "Unable to save the image.",

			#[cfg(feature = "menu")]
			Self::Menu(e) => e.as_str(),
		}
	}
}
