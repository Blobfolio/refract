/*!
# `Refract` - Encoded Image
*/

use crate::{
	Candidate,
	OutputKind,
	RefractError,
};
use std::{
	borrow::Cow,
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
	pub fn update(&mut self, src: &Candidate) -> Result<(), RefractError> {
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
	/// # Kind.
	pub const fn kind(&self) -> OutputKind { self.kind }

	#[must_use]
	/// # Formatted Quality.
	///
	/// This returns the quality as a string, formatted according to the type
	/// and value.
	pub fn nice_quality(&self) -> Cow<str> {
		// Lossless.
		if self.quality == self.kind.lossless_quality() {
			Cow::Borrowed("lossless quality")
		}
		// Weird AVIF.
		else if self.kind == OutputKind::Avif {
			Cow::Owned(format!("quantizer {:.1}", 63 - self.quality.get()))
		}
		// Weird JPEG XL.
		else if self.kind == OutputKind::Jxl {
			let f_quality = f32::from(150_u8 - self.quality.get()) / 10.0;
			Cow::Owned(format!("quality {:.1}", f_quality))
		}
		// It is what it is.
		else {
			Cow::Owned(format!("quality {}", self.quality))
		}
	}

	#[must_use]
	/// # Quality.
	pub const fn quality(&self) -> NonZeroU8 { self.quality }

	#[must_use]
	/// # Size.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}
