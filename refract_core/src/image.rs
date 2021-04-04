/*!
# `Refract`: Image
*/

use crate::Avif;
use crate::ImageKind;
use crate::RefractError;
use crate::Refraction;
use crate::Webp;
use std::convert::TryFrom;
use std::num::NonZeroU64;
use std::path::PathBuf;



#[derive(Debug)]
/// # Image.
pub struct Image<'a> {
	src: &'a PathBuf,
	kind: ImageKind,
	size: NonZeroU64,
}

impl<'a> TryFrom<&'a PathBuf> for Image<'a> {
	type Error = RefractError;

	fn try_from(file: &'a PathBuf) -> Result<Self, Self::Error> {
		Ok(Self {
			src: file,
			kind: ImageKind::try_from(file)?,
			size: NonZeroU64::new(std::fs::metadata(file).map_or(0, |m| m.len()))
				.ok_or(RefractError::InvalidImage)?,
		})
	}
}

impl<'a> Image<'a> {
	#[inline]
	/// # Try `AVIF`.
	///
	/// Try to find an acceptable `AVIF` version of the image.
	///
	/// ## Errors
	///
	/// This method returns an error if no acceptable image is found, either
	/// because they all looked terrible or were larger than the source.
	pub fn try_avif(&self) -> Result<Refraction, RefractError> {
		Avif::new(self).find()
	}

	#[inline]
	/// # Try `WebP`.
	///
	/// Try to find an acceptable `WebP` version of the image.
	///
	/// ## Errors
	///
	/// This method returns an error if no acceptable image is found, either
	/// because they all looked terrible or were larger than the source.
	pub fn try_webp(&self) -> Result<Refraction, RefractError> {
		Webp::new(self).find()
	}

	#[must_use]
	/// # Path.
	pub const fn path(&self) -> &PathBuf { self.src }

	#[must_use]
	/// # Kind.
	pub const fn kind(&self) -> ImageKind { self.kind }

	#[must_use]
	/// # Size.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}
