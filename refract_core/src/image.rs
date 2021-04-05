/*!
# `Refract`: Image
*/

use crate::{
	Avif,
	ImageKind,
	RefractError,
	Refraction,
	Webp,
};
use imgref::ImgVec;
use ravif::{
	Img,
	RGBA8,
};
use std::{
	convert::TryFrom,
	num::NonZeroU64,
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # Image.
///
/// This holds data for a source image — either a JPEG or PNG. It is
/// instantiated with a reference to a `PathBuf`, and lives as long as that
/// reference.
pub struct Image<'a> {
	src: &'a PathBuf,
	img: ImgVec<RGBA8>,
	kind: ImageKind,
	size: NonZeroU64,
}

impl<'a> TryFrom<&'a PathBuf> for Image<'a> {
	type Error = RefractError;

	fn try_from(file: &'a PathBuf) -> Result<Self, Self::Error> {
		let raw = std::fs::read(file)
			.map_err(|_| RefractError::InvalidImage)?;

		Ok(Self {
			src: file,
			img: crate::load_rgba(&raw)?,
			kind: ImageKind::try_from(raw.as_slice())?,
			size: NonZeroU64::new(u64::try_from(raw.len()).map_err(|_| RefractError::InvalidImage)?)
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
	pub fn try_avif(&mut self) -> Result<Refraction, RefractError> {
		self.img = ravif::cleared_alpha(self.img.clone());
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
	///
	/// Returns a reference to the source's file system path.
	pub const fn path(&self) -> &PathBuf { self.src }

	#[must_use]
	/// # Image.
	///
	/// Returns a reference to the image.
	pub fn img(&self) -> Img<&[RGBA8]> { self.img.as_ref() }

	#[must_use]
	/// # Kind.
	///
	/// Returns the kind of image.
	pub const fn kind(&self) -> ImageKind { self.kind }

	#[must_use]
	/// # Size.
	///
	/// Returns the disk size of the image (in bytes).
	pub const fn size(&self) -> NonZeroU64 { self.size }
}
