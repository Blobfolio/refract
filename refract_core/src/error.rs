/*!
# `Refract` - Error
*/

#[cfg(feature = "bin")] use argyle::ArgyleError;
use crate::ImageKind;
use std::{
	error::Error,
	fmt,
};



#[allow(missing_docs)]
#[derive(Debug, Copy, Clone)]
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
	TooDissimilar,

	#[cfg(feature = "bin")]
	/// # Passthrough menu error.
	Menu(ArgyleError),

	#[cfg(feature = "bin")]
	NoCompression,

	#[cfg(feature = "bin")]
	NoEncoders,

	#[cfg(feature = "bin")]
	NoImages,

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
			Self::TooDissimilar => "The DSSIM for the encoded image is too big.",

			#[cfg(feature = "bin")]
			Self::Menu(e) => e.as_str(),
			#[cfg(feature = "bin")]
			Self::NoCompression => "Lossless and lossy encoding cannot both be disabled.",
			#[cfg(feature = "bin")]
			Self::NoEncoders => "At least one encoder must be enabled.",
			#[cfg(feature = "bin")]
			Self::NoImages => "No images were found.",
			#[cfg(feature = "bin")]
			Self::Read => "Unable to read the source file.",
			#[cfg(feature = "bin")]
			Self::Write => "Unable to save the file.",
		}
	}
}
