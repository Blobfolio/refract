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



const FLAG_KIND: u8        = 0b0001;
const FLAG_SIZE: u8        = 0b0010;
const FLAG_MUT_BORROW: u8  = 0b0100;
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
	pub const fn new(kind: OutputKind) -> Self {
		Self {
			buf: Vec::new(),
			quality: kind.lossless_quality(),
			kind,
			flags: 0,
		}
	}

	/// # Reset.
	pub fn reset(&mut self) {
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
	pub fn verify(&mut self, size: NonZeroU64) -> Result<(), RefractError> {
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
	pub fn as_mut_vec(&mut self) -> &mut Vec<u8> { &mut self.buf }

	#[must_use]
	/// # Is Verified?
	pub const fn is_verified(&self) -> bool { self.flags == FLAG_VALID }

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
	pub fn finish_mut_vec(&mut self) { self.flags = 0; }

	/// # From Quality.
	pub fn set_quality(&mut self, quality: Option<NonZeroU8>) {
		self.reset();
		if let Some(quality) = quality {
			self.quality = quality;
		}
		else {
			self.quality = self.kind.lossless_quality();
		}
	}

	/// # From Slice.
	pub fn set_slice(&mut self, data: &[u8]) {
		self.reset();
		self.buf.extend_from_slice(data);
	}
}
