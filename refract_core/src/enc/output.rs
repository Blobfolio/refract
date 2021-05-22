/*!
# `Refract` - Encoded Candidate Image.
*/

use crate::{
	FLAG_VALID,
	ImageKind,
	Quality,
	RefractError,
};
use std::{
	convert::TryFrom,
	num::NonZeroUsize,
	ops::Deref,
};



#[derive(Debug)]
/// # Output Image.
///
/// This struct holds the raw file data for an encoded image along with
/// information about the quality used to create it.
///
/// This is used by [`EncodeIter`] for both preview/candidate images and the
/// final "best". It cannot be instantiated independently of the guided
/// iterator.
///
/// Both `AsRef<[u8]>` and `Deref` traits are implemented to provide raw access
/// to the data.
pub struct Output {
	data: Vec<u8>,
	quality: Quality,
	flags: u8,
}

impl AsRef<[u8]> for Output {
	#[inline]
	fn as_ref(&self) -> &[u8] { self }
}

impl Deref for Output {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target {
		if self.is_valid() { &self.data }
		else { &[] }
	}
}

/// ## Instantiation.
impl Output {
	#[inline]
	#[must_use]
	/// # New.
	///
	/// Create a new, empty instance expecting the provided output kind.
	pub(crate) const fn new(kind: ImageKind) -> Self {
		Self {
			data: Vec::new(),
			quality: Quality::Lossless(kind),
			flags: 0,
		}
	}

	/// # Reset.
	///
	/// The [`EncodeIter`] struct minimizes allocations by reusing candidate
	/// buffers between writes. This resets the state so it can be used again.
	fn reset(&mut self) {
		self.data.truncate(0);
		self.flags = 0;
	}

	/// # Finish (and Validate).
	///
	/// This method is called after the encoder has written data to the buffer
	/// to ensure the data is correct and is smaller than the reference size.
	///
	/// Validity is used as a sanity guard when trying to obtain the bytes.
	/// Insofar as is possible, it will refuse to release garbage.
	///
	/// ## Errors
	///
	/// This will return an error if the data does not match the expected
	/// output format or if the size is too large.
	pub(crate) fn finish(&mut self, size: usize) -> Result<(), RefractError> {
		// If for some reason this has already been called, pass through the
		// answer.
		if self.is_valid() { Ok(()) }
		else {
			// Check the length. That's pretty easy.
			let len: usize = self.data.len();
			if size == 0 || len == 0 {
				Err(RefractError::Encode)
			}
			else if len >= size {
				Err(RefractError::TooBig)
			}
			// We're good! Probably.
			else if self.quality.kind() == ImageKind::try_from(self.data.as_slice())? {
				self.flags |= FLAG_VALID;
				Ok(())
			}
			// Type mismatch.
			else {
				Err(RefractError::Encode)
			}
		}
	}
}

/// ## Getters.
impl Output {
	#[inline]
	/// # As Mut Vec.
	///
	/// This is used internally by the JPEG XL encoder to stream write the
	/// results.
	pub(crate) fn as_mut_vec(&mut self) -> &mut Vec<u8> { &mut self.data }

	#[inline]
	#[must_use]
	/// # Flags.
	///
	/// This returns the [`EncodeIter`] flags that were set when the data was
	/// written.
	///
	/// At the moment, this is only useful for identifying whether an AVIF was
	/// encoded in full- or limited-range, but may hold other information in
	/// the future.
	///
	/// Note: a value is returned even in cases where the data itself wound up
	/// invalid.
	pub const fn flags(&self) -> u8 { self.flags }

	#[inline]
	#[must_use]
	/// # Is Valid?
	///
	/// This method shouldn't need to be called by external libraries, but can
	/// be, I suppose.
	///
	/// In practice, if [`EncodeIter`] returns an [`Output`] reference, it _is_
	/// valid. Otherwise it will just return an error.
	pub const fn is_valid(&self) -> bool { FLAG_VALID == self.flags & FLAG_VALID }

	#[inline]
	#[must_use]
	/// # Kind.
	///
	/// This is a pass-through method for returning the underlying image
	/// format.
	///
	/// Note: a value is returned even in cases where the data itself wound up
	/// invalid.
	pub const fn kind(&self) -> ImageKind { self.quality.kind() }

	#[inline]
	#[must_use]
	/// # Quality.
	///
	/// This returns the quality used when encoding the data.
	///
	/// Note: a value is returned even in cases where the data itself wound up
	/// invalid.
	pub const fn quality(&self) -> Quality { self.quality }

	#[inline]
	#[must_use]
	/// # Size.
	///
	/// Return the byte size of the image, or `None` if there isn't one.
	///
	/// To prevent weirdness, a size is only returned if the image data is
	/// valid.
	pub fn size(&self) -> Option<NonZeroUsize> {
		if self.is_valid() {
			NonZeroUsize::new(self.data.len())
		}
		else { None }
	}
}

/// ## Setters.
impl Output {
	/// # Copy To.
	///
	/// The [`EncodeIter`] struct stores two instances of [`Output`]: one for
	/// candidate images, and one for the "best" found.
	///
	/// This is used to replace the old best with a new best, minimizing
	/// reallocation as much as possible.
	///
	/// Note: this will reset `self` in the process.
	pub(crate) fn copy_to(&mut self, dst: &mut Self) {
		dst.quality = self.quality;
		dst.flags = self.flags;
		dst.data.truncate(0);
		dst.data.append(&mut self.data);
	}

	/// # Set Target Quality and Flags.
	///
	/// This resets the buffer and updates the quality, kind, and/or flags.
	///
	/// This method is always called prior to writing any new data, and these
	/// values will persist even in cases where the data write fails.
	pub(crate) fn set_quality(&mut self, quality: Quality, flags: u8) {
		self.reset();
		self.flags = flags;
		self.quality = quality;
	}

	/// # Set Data From Slice.
	///
	/// This method shoves the raw byte slice returned by the `WebP` and `AVIF`
	/// encoders into permanent storage.
	///
	/// [`EncodeIter`] will call [`Output::finish`] afterwards to validate the
	/// data written.
	pub(crate) fn set_slice(&mut self, data: &[u8]) {
		if self.data.is_empty() {
			self.data.extend_from_slice(data);
		}
	}
}
