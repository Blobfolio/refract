/*!
# `Refract`: Image Candidate
*/

use crate::{
	Encoder,
	RefractError,
	Refraction,
};
use ravif::{
	Img,
	RGBA8,
};
use std::{
	ffi::OsStr,
	fs::File,
	num::{
		NonZeroU64,
		NonZeroU8,
	},
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



#[derive(Debug)]
/// # Image Candidate
///
/// This holds information for an in-progress candidate image.
pub struct Candidate<'a> {
	/// # The output path.
	dst: PathBuf,
	/// # The size of the output image, if any.
	dst_size: Option<NonZeroU64>,
	/// # The quality used to encode the output image, if any.
	dst_quality: Option<NonZeroU8>,
	/// # A temporary path for preview images.
	tmp: PathBuf,
	/// # The source image pixels.
	img: Img<&'a [RGBA8]>,
}

impl<'a> Candidate<'a> {
	#[allow(trivial_casts)] // Triviality is necessary.
	#[must_use]
	/// # New.
	///
	/// Start a new instance given a path, image, and encoder.
	pub fn new(src: &Path, img: Img<&'a [RGBA8]>, enc: Encoder) -> Self {
		// The distribution and temporary paths are derived from the source
		// path. Doing this from bytes is a lot more efficient than using `Path`
		// methods.
		let stub: &[u8] = unsafe { &*(src.as_os_str() as *const OsStr as *const [u8]) };

		Self {
			dst: PathBuf::from(OsStr::from_bytes(&[stub, enc.ext()].concat())),
			dst_size: None,
			dst_quality: None,
			tmp: PathBuf::from(OsStr::from_bytes(&[stub, b".PROPOSED", enc.ext()].concat())),
			img
		}
	}

	/// # Clean.
	///
	/// This will delete the temporary image, if it exists, and the
	/// distribution image if its size and/or quality are missing.
	///
	/// ## Errors
	///
	/// This will return an error if the disk changes are unable to be written.
	pub fn clean(&self) -> Result<(), RefractError> {
		if self.tmp.exists() {
			std::fs::remove_file(&self.tmp)
				.map_err(|_| RefractError::Write)?;
		}

		if (self.dst_size.is_none() || self.dst_quality.is_none()) && self.dst.exists() {
			std::fs::remove_file(&self.dst)
				.map_err(|_| RefractError::Write)?;
		}

		Ok(())
	}

	/// # Keep Temporary Image.
	///
	/// This will move the temporary image to the distribution path and record
	/// the provided size and quality.
	///
	/// ## Errors
	///
	/// This will return an error if the disk changes are unable to be written
	/// due to permission errors, missing source, etc.
	pub fn keep(&mut self, size: NonZeroU64, quality: NonZeroU8) -> Result<(), RefractError> {
		std::fs::rename(&self.tmp, &self.dst)
			.map_err(|_| RefractError::Write)?;

		self.set_size_quality(size, quality);

		Ok(())
	}

	/// # Set Output Size/Quality.
	///
	/// This is a shorthand method to update the distribution size and quality.
	/// The distribution image must exist, or these values won't end up meaning
	/// anything.
	pub(crate) fn set_size_quality(&mut self, size: NonZeroU64, quality: NonZeroU8) {
		self.dst_size.replace(size);
		self.dst_quality.replace(quality);
	}

	/// # Take Or.
	///
	/// This method consumes the [`Candidate`], returning either a [`Refraction`]
	/// instance if one was found.
	///
	/// ## Errors
	///
	/// If the distribution image does not exist or if either its size or
	/// quality are undefined, the provided `err` is passed through instead.
	pub fn take_or(self, err: RefractError) -> Result<Refraction, RefractError> {
		if self.dst.exists() {
			if let Some((size, quality)) = self.dst_size.zip(self.dst_quality) {
				return Ok(Refraction::new(self.dst, size, quality));
			}
		}

		Err(err)
	}

	#[inline]
	/// # Write Output Image.
	///
	/// This is a convenience method for writing image data to the
	/// output path.
	///
	/// ## Errors
	///
	/// An error is returned if the data cannot be written to disk.
	pub(crate) fn write_dst(&self, data: &[u8]) -> Result<(), RefractError> {
		write_img(&self.dst, data)
	}

	#[inline]
	/// # Write Tmp Image.
	///
	/// This is a convenience method for writing image data to the
	/// temporary path.
	///
	/// ## Errors
	///
	/// An error is returned if the data cannot be written to disk.
	pub(crate) fn write_tmp(&self, data: &[u8]) -> Result<(), RefractError> {
		write_img(&self.tmp, data)
	}
}

impl<'a> Candidate<'a> {
	#[must_use]
	/// Is Smaller?
	///
	/// Check if a given size is smaller than the current best. If there is no
	/// current best, `true` is returned.
	pub fn is_smaller(&self, size: NonZeroU64) -> bool {
		self.dst_size.map_or(true, |s| size < s)
	}

	#[must_use]
	/// # Image.
	pub const fn img(&self) -> Img<&'a [RGBA8]> { self.img }

	#[must_use]
	/// # Output Path.
	pub const fn dst_path(&self) -> &PathBuf { &self.dst }

	#[must_use]
	/// # Temporary Path.
	pub const fn tmp_path(&self) -> &PathBuf { &self.tmp }
}



/// # Write File.
fn write_img(path: &Path, data: &[u8]) -> Result<(), RefractError> {
	use std::io::Write;

	File::create(path)
		.and_then(|mut file| file.write_all(data).and_then(|_| file.flush()))
		.map_err(|_| RefractError::Write)
}
