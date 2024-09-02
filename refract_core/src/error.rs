/*!
# `Refract` - Error
*/

use crate::ImageKind;
use std::{
	error::Error,
	fmt,
};



#[expect(missing_docs, reason = "Self-explanatory.")]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Errors.
pub enum RefractError {
	Color,
	Decode,
	Encode,
	Image,
	ImageDecode(ImageKind),
	ImageEncode(ImageKind),
	NoBest(ImageKind),
	NothingDoing,
	Overflow,
	TooBig,

	#[cfg(feature = "bin")]
	Argue(argyle::ArgyleError),

	#[cfg(feature = "bin")]
	GtkInit,

	#[cfg(feature = "bin")]
	MissingSource,

	#[cfg(feature = "bin")]
	NoEncoders,

	#[cfg(feature = "bin")]
	NoSave,

	#[cfg(feature = "bin")]
	Read,

	#[cfg(feature = "bin")]
	Write,
}

impl Error for RefractError {}

impl AsRef<str> for RefractError {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

#[cfg(feature = "bin")]
impl From<argyle::ArgyleError> for RefractError {
	#[inline]
	fn from(err: argyle::ArgyleError) -> Self { Self::Argue(err) }
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
			Self::Color => "Unsupported color encoding format.",
			Self::Decode => "The image could not be decoded.",
			Self::Encode => "The image could not be encoded.",
			Self::Image => "Invalid image.",
			Self::ImageDecode(k) => match k {
				ImageKind::Avif => "Refract cannot decode AVIF images.",
				ImageKind::Jxl => "Refract cannot decode JPEG XL images.",
				ImageKind::Webp => "Refract cannot decode WebP images.",
				_ => "",
			},
			Self::ImageEncode(k) => match k {
				ImageKind::Jpeg => "Refract cannot encode JPEG files.",
				ImageKind::Png => "Refract cannot encode PNG files.",
				_ => "",
			},
			Self::NoBest(k) => match k {
				ImageKind::Avif => "No acceptable AVIF candidate was found.",
				ImageKind::Jxl => "No acceptable JPEG XL candidate was found.",
				ImageKind::Webp => "No acceptable WebP candidate was found.",
				_ => "",
			},
			Self::NothingDoing => "There is nothing else to do.",
			Self::Overflow => "The image dimensions are out of range.",
			Self::TooBig => "The encoded image was too big.",

			#[cfg(feature = "bin")]
			Self::Argue(a) => a.as_str(),

			#[cfg(feature = "bin")]
			Self::GtkInit => "Failed to initialize GTK.",

			#[cfg(feature = "bin")]
			Self::MissingSource => "A source image must be set before a candidate image.",

			#[cfg(feature = "bin")]
			Self::NoEncoders => "At least one encoder must be enabled.",

			#[cfg(feature = "bin")]
			Self::NoSave => "The result was not saved.",

			#[cfg(feature = "bin")]
			Self::Read => "Unable to read the source file.",

			#[cfg(feature = "bin")]
			Self::Write => "Unable to save the file.",
		}
	}
}
