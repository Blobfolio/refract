/*!
# `Refract` - Encoded Image
*/

use crate::OutputKind;
use crate::RefractError;
use std::borrow::Cow;
use std::convert::TryFrom;
use std::num::NonZeroU64;
use std::num::NonZeroU8;
use std::ops::Deref;



#[derive(Debug, Clone)]
/// # Encoded Image.
///
/// This holds the raw data for an encoded image along with basic metadata.
pub struct Output {
	raw: Box<[u8]>,
	size: NonZeroU64,
	quality: NonZeroU8,
	kind: OutputKind,
}

impl Deref for Output {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target { self.raw.as_ref() }
}

/// ## Construction.
impl Output {
	/// # New Instance.
	///
	/// Create a new instance from raw bytes and quality.
	///
	/// ## Errors
	///
	/// This will return an error if the image is invalid or its size overflows.
	pub fn new(raw: Box<[u8]>, quality: NonZeroU8) -> Result<Self, RefractError> {
		let kind = OutputKind::try_from(raw.as_ref())?;

		// We know this is non-zero because we were able to obtain a valid
		// image kind from its headers.
		let size = u64::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;
		let size = unsafe { NonZeroU64::new_unchecked(size) };

		Ok(Self {
			raw,
			size,
			quality,
			kind,
		})
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
