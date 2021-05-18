/*!
# `Refract` - Encoded Image
*/

use crate::{
	Candidate,
	FLAG_LOSSLESS,
	OutputKind,
	Quality,
	RefractError,
};
use std::{
	convert::TryFrom,
	num::{
		NonZeroU64,
		NonZeroU8,
	},
	ops::Deref,
};



#[derive(Debug, Clone)]
/// # Encoded Image.
///
/// This holds the raw data for an encoded image along with basic metadata.
pub struct Output {
	raw: Vec<u8>,
	size: NonZeroU64,
	quality: NonZeroU8,
	kind: OutputKind,
	flags: u8,
}

impl Deref for Output {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target { self.raw.as_ref() }
}

impl TryFrom<&Candidate> for Output {
	type Error = RefractError;

	fn try_from(src: &Candidate) -> Result<Self, Self::Error> {
		let raw = src.as_slice()?.to_vec();
		let size = u64::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;
		Ok(Self {
			raw,
			size: unsafe { NonZeroU64::new_unchecked(size) },
			quality: src.quality(),
			kind: src.kind(),
			flags: 0,
		})
	}
}

/// ## Construction.
impl Output {
	/// # Update.
	///
	/// Replace the inner bits with new data. This can save a few allocations.
	///
	/// ## Errors
	///
	/// This will return an error if the image is invalid or its size overflows.
	pub(crate) fn update(&mut self, src: &Candidate) -> Result<(), RefractError> {
		let raw = src.as_slice()?;
		let size = u64::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;

		// A few additional sanity checks.
		if src.kind() != self.kind || size > self.size.get() {
			return Err(RefractError::Encode);
		}

		self.raw.truncate(0);
		self.raw.extend_from_slice(raw);

		self.size = unsafe { NonZeroU64::new_unchecked(size) };
		self.quality = src.quality();

		Ok(())
	}
}

/// # Getters.
impl Output {
	#[must_use]
	/// # Flags.
	pub const fn flags(&self) -> u8 { self.flags }

	#[must_use]
	/// # Kind.
	pub const fn kind(&self) -> OutputKind { self.kind }

	#[must_use]
	/// # Lossless.
	pub const fn lossless(&self) -> bool { FLAG_LOSSLESS == self.flags & FLAG_LOSSLESS }

	#[must_use]
	/// # Formatted Quality.
	///
	/// This returns the quality as a string, formatted according to the type
	/// and value.
	pub const fn nice_quality(&self) -> Quality {
		if self.lossless() { Quality::Lossless(self.kind) }
		else {
			Quality::Lossy(self.kind, self.quality)
		}
	}

	#[must_use]
	/// # Quality.
	pub const fn quality(&self) -> NonZeroU8 { self.quality }

	#[must_use]
	/// # Size.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}

/// # Setters.
impl Output {
	/// # Set Flags.
	pub(crate) fn set_flags(&mut self, flags: u8) {
		self.flags = flags;
	}
}
