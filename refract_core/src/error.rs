/*!
# `Refract` - Error
*/

use crate::OutputKind;
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
			Self::Candidate(kind) => match kind {
				OutputKind::Avif => "No acceptable AVIF candidate was found.",
				OutputKind::Webp => "No acceptable WebP candidate was found.",
			},
			Self::Encode => "Errors were encountered while trying to encode the image.",
			Self::NoLossless => "Lossless encoding is not supported.",
			Self::Read => "Unable to read the source image.",
			Self::Source => "Invalid image source.",
			Self::TooBig => "The encoded image was larger than the source.",
			Self::Write => "Unable to save the image.",
		}
	}
}
