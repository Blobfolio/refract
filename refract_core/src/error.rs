/*!
# `Refract` - Error
*/

#[cfg(feature = "menu")] use argyle::ArgyleError;
use crate::OutputKind;
use std::{
	error::Error,
	fmt,
};



#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
/// # Error.
pub enum RefractError {
	Color,
	Encode,
	NoBest(OutputKind),
	NoEncoders,
	NoImages,
	NoLosslessLossy,
	NothingDoing,
	Output,
	Overflow,
	Read,
	Source,
	TooBig,
	Write,

	#[cfg(feature = "menu")]
	/// # Passthrough menu error.
	Menu(ArgyleError),
}

impl Error for RefractError {}

impl AsRef<str> for RefractError {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for RefractError {
	#[inline]
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
			Self::Color => "Unsupported color encoding.",
			Self::Encode => "Unable to encode the image.",
			Self::NoBest(k) => match k {
				OutputKind::Avif => "No acceptable AVIF candidate was found.",
				OutputKind::Jxl => "No acceptable JPEG XL candidate was found.",
				OutputKind::Webp => "No acceptable WebP candidate was found.",
			},
			Self::NoEncoders => "No encoders were selected.",
			Self::NoImages => "No images were found.",
			Self::NoLosslessLossy => "Lossless and lossy cannot both be disabled. Haha.",
			Self::NothingDoing => "There is nothing else to do.",
			Self::Output => "Invalid output format.",
			Self::Overflow => "The numeric value is out of range.",
			Self::Read => "Unable to read the source file.",
			Self::Source => "Invalid source image.",
			Self::TooBig => "The encoded image was too big.",
			Self::Write => "Unable to save the file.",

			#[cfg(feature = "menu")]
			Self::Menu(e) => e.as_str(),
		}
	}
}
