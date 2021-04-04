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
	raw: Box<[u8]>,
	kind: ImageKind,
	size: NonZeroU64,
}

impl<'a> TryFrom<&'a PathBuf> for Image<'a> {
	type Error = RefractError;

	fn try_from(file: &'a PathBuf) -> Result<Self, Self::Error> {
		let raw = std::fs::read(file)
			.map_err(|_| RefractError::InvalidImage)?
			.into_boxed_slice();

		Ok(Self {
			src: file,
			kind: ImageKind::try_from(raw.as_ref())?,
			raw,
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
	/// # Raw.
	pub const fn raw(&self) -> &[u8] { &self.raw }

	#[must_use]
	/// # Kind.
	pub const fn kind(&self) -> ImageKind { self.kind }

	#[must_use]
	/// # Size.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}
