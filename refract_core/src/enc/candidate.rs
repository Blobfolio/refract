/*!
# `Refract` - Encoded Candidate
*/

use crate::{
	OutputKind,
	RefractError,
};
use std::{
	convert::TryFrom,
	num::{
		NonZeroU64,
		NonZeroU8,
	},
};



/// # Flag: Kind Validated.
const FLAG_KIND: u8        = 0b0001;

/// # Flag: Size Validated.
const FLAG_SIZE: u8        = 0b0010;

/// # Flag: Mutable Vec is Outstanding.
const FLAG_MUT_BORROW: u8  = 0b0100;

/// # Flag: All validated.
const FLAG_VALID: u8       = 0b0011;



#[derive(Debug, Clone)]
/// # Candidate Image.
///
/// This holds a buffer the encoder can write to and information about the
/// quality, etc.
///
/// Most of the allocations happen in FFI land, but we can save some memory I/O
/// and repetitive code by sharing buffers across writes.
pub struct Candidate {
	buf: Vec<u8>,
	quality: NonZeroU8,
	kind: OutputKind,
	flags: u8,
}

/// # Instantiation.
impl Candidate {
	#[must_use]
	/// # New.
	pub(crate) const fn new(kind: OutputKind) -> Self {
		Self {
			buf: Vec::new(),
			quality: kind.lossless_quality(),
			kind,
			flags: 0,
		}
	}

	/// # Reset.
	///
	/// This lets us reuse the same instance for multiple passes, saving a few
	/// allocations.
	pub(crate) fn reset(&mut self) {
		self.buf.truncate(0);
		self.flags = 0;
	}

	/// # Verify Candidate.
	///
	/// This checks the buffer and/or size to make sure it is acceptable.
	///
	/// ## Errors
	///
	/// An error will be returned — and the struct reset — if validation fails.
	pub(crate) fn verify(&mut self, size: NonZeroU64) -> Result<(), RefractError> {
		if ! self.verify_kind() || 0 != self.flags & FLAG_MUT_BORROW {
			self.reset();
			return Err(RefractError::Encode);
		}

		if ! self.verify_size(size) {
			self.reset();
			return Err(RefractError::TooBig);
		}

		Ok(())
	}

	/// # Verify Kind.
	///
	/// This checks the data kind matches the expected [`OutputKind`].
	fn verify_kind(&mut self) -> bool {
		if 0 != self.flags & FLAG_KIND { return true; }
		else if let Ok(kind) = OutputKind::try_from(self.buf.as_slice()) {
			if kind == self.kind {
				self.flags |= FLAG_KIND;
				return true;
			}
		}

		false
	}

	/// # Verify Size.
	///
	/// This checks the size of the data is smaller than the "best" found to
	/// date, passed from the outside.
	fn verify_size(&mut self, size: NonZeroU64) -> bool {
		if 0 != self.flags & FLAG_SIZE { return true; }
		else if let Ok(buf_size) = u64::try_from(self.buf.len()) {
			if 0 < buf_size && buf_size < size.get() {
				self.flags |= FLAG_SIZE;
				return true;
			}

		}

		false
	}
}

/// # Getters.
impl Candidate {
	/// # As Slice.
	///
	/// This shouldn't normally fail, but if someone calls it early there may
	/// be problems.
	///
	/// ## Errors
	///
	/// This will return an error if the candidate has not been verified with
	/// [`Candidate::verify`].
	pub fn as_slice(&self) -> Result<&[u8], RefractError> {
		if self.is_verified() { Ok(self.buf.as_slice()) }
		else { Err(RefractError::Encode) }
	}

	/// # As Mut Vec.
	///
	/// Return a mutable reference to the underlying buffer.
	///
	/// For safety, [`Candidate::finish_mut_vec`] must be called after writing
	/// is complete or it will fail validation.
	///
	/// It still isn't fully safe, but prevents cases where a mid-stage abort
	/// causes data to only be partially written.
	///
	/// On the bright side, this is only used by JPEG XL, which requires a
	/// specific end byte, so type validation would generally fail if writes
	/// don't finish.
	pub(crate) fn as_mut_vec(&mut self) -> &mut Vec<u8> { &mut self.buf }

	#[must_use]
	/// # Is Verified?
	pub(crate) const fn is_verified(&self) -> bool { self.flags == FLAG_VALID }

	#[must_use]
	/// # Kind.
	pub const fn kind(&self) -> OutputKind { self.kind }

	#[must_use]
	/// # Quality.
	pub const fn quality(&self) -> NonZeroU8 { self.quality }
}

/// # Setters.
impl Candidate {
	/// # Finish Mut Vec.
	///
	/// This method must be called after borrowing a mutable vec or the result
	/// will fail validation.
	pub(crate) fn finish_mut_vec(&mut self) { self.flags = 0; }

	/// # From Quality.
	pub(crate) fn set_quality(&mut self, quality: Option<NonZeroU8>) {
		self.reset();
		if let Some(quality) = quality {
			self.quality = quality;
		}
		else {
			self.quality = self.kind.lossless_quality();
		}
	}

	/// # From Slice.
	pub(crate) fn set_slice(&mut self, data: &[u8]) {
		self.reset();
		self.buf.extend_from_slice(data);
	}
}
