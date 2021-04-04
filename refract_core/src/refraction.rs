/*!
# `Refract`: Refracted Image
*/

use std::num::NonZeroU8;
use std::num::NonZeroU64;
use std::path::PathBuf;
use std::borrow::Cow;



#[derive(Debug)]
/// # Image Result.
pub struct Refraction {
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
	pub fn new(path: PathBuf, size: NonZeroU64, quality: NonZeroU8) -> Self {
		assert!(path.file_name().is_some());

		Self { path, size, quality }
	}

	#[must_use]
	/// # Path.
	pub const fn path(&self) -> &PathBuf { &self.path }

	/// # File name.
	///
	/// ## Panics
	///
	/// This will panic if the struct was instantiated with an invalid path.
	/// This crate won't do that, so it should be fine, but if using this
	/// externally, make sure there is a file name.
	#[must_use]
	pub fn name(&self) -> Cow<str> {
		self.path.file_name().unwrap().to_string_lossy()
	}

	#[must_use]
	/// # Quality.
	pub const fn quality(&self) -> NonZeroU8 { self.quality }

	#[must_use]
	/// # File Size.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}
