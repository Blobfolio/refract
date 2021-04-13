/*!
# `Refract`: Refracted Image
*/

use std::{
	borrow::Cow,
	num::{
		NonZeroU8,
		NonZeroU64,
	},
	path::PathBuf,
};



#[derive(Debug)]
/// # Image Result.
///
/// This holds the information for a generated `WebP` or `AVIF` image, namely
/// its path, size, and the quality setting used.
///
/// This struct is only instantiated if conversion is successful.
pub(super) struct Refraction {
	path: PathBuf,
	size: NonZeroU64,
	quality: NonZeroU8,
}

impl Refraction {
	#[must_use]
	/// # New.
	///
	/// ## Panics
	///
	/// This will panic if the path does not include a file name. When set by
	/// the methods in this crate it will, but if used externally, be careful!
	pub(crate) fn new(path: PathBuf, size: NonZeroU64, quality: NonZeroU8) -> Self {
		assert!(path.file_name().is_some());
		Self { path, size, quality }
	}

	/// # File name.
	///
	/// ## Panics
	///
	/// This will technically panic in cases where there is no file name
	/// component to the path, however instantiation already checks that
	/// assertion, so it shouldn't panic here.
	#[must_use]
	pub(crate) fn name(&self) -> Cow<str> {
		self.path.file_name().unwrap().to_string_lossy()
	}

	#[must_use]
	/// # Quality.
	///
	/// This returns the quality setting (`1..=100`) used when creating the
	/// image. A value of `100` indicates `lossless`, but only applies to
	/// `WebP`.
	pub(crate) const fn quality(&self) -> NonZeroU8 { self.quality }

	#[must_use]
	/// # File Size.
	///
	/// Return the file size in bytes.
	pub(crate) const fn size(&self) -> NonZeroU64 { self.size }
}
