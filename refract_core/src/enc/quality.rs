/*!
# `Refract` - Encoding Quality.
*/

use crate::ImageKind;
use std::{
	fmt,
	num::NonZeroU8,
};



#[derive(Debug, Clone, Copy)]
/// # Encoding Quality.
///
/// Refract internally uses `NonZeroU8` values to represent encoding qualities,
/// but individual encoders have their own ideas about how things should be.
///
/// This enum provides a consistent interface for everyone to work with.
pub enum Quality {
	/// # Lossless.
	Lossless(ImageKind),
	/// # Lossy.
	Lossy(ImageKind, NonZeroU8),
}

impl fmt::Display for Quality {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Lossless(_) => f.write_str("lossless quality"),
			Self::Lossy(_, _) => write!(f, "{} {}", self.label(), self.quality()),
		}
	}
}

/// ## Instantiation.
impl Quality {
	#[inline]
	#[must_use]
	/// # New.
	pub(crate) fn new(kind: ImageKind, quality: Option<NonZeroU8>) -> Self {
		quality.map_or_else(|| Self::Lossless(kind), |q| Self::Lossy(kind, q))
	}
}

/// ## Getters.
impl Quality {
	#[must_use]
	/// # Is Lossless?
	pub const fn is_lossless(self) -> bool { matches!(self, Self::Lossless(_)) }

	#[must_use]
	/// # Kind.
	///
	/// This is a pass-through method for returning the output image format
	/// associated with the value.
	pub const fn kind(self) -> ImageKind {
		match self {
			Self::Lossless(k) | Self::Lossy(k, _) => k,
		}
	}

	#[must_use]
	/// # Label.
	///
	/// This returns the word used by the encoder to signify quality, because
	/// encoders can't even agree on that. Haha.
	///
	/// At the moment, AVIF calls it "quantizer", but everyone else calls it
	/// "quality".
	pub const fn label(self) -> &'static str {
		match self.kind() {
			ImageKind::Avif => "quantizer",
			_ => "quality",
		}
	}

	#[must_use]
	/// # Label (Title Case).
	///
	/// This is the same as [`Quality::label`] except it is in title case
	/// rather than lower case.
	pub const fn label_title(self) -> &'static str {
		match self.kind() {
			ImageKind::Avif => "Quantizer",
			_ => "Quality",
		}
	}

	#[must_use]
	/// # Normalized Quality Value.
	///
	/// This returns the quality value in the native format of the encoder. See
	/// [`QualityValue`] for more information.
	pub fn quality(self) -> QualityValue {
		match self {
			Self::Lossless(_) => QualityValue::Lossless,
			Self::Lossy(k, q) => match k {
				ImageKind::Avif => QualityValue::Int(63 - q.get()),
				ImageKind::Jxl => QualityValue::Float(f32::from(150_u8 - q.get()) / 10.0),
				_ => QualityValue::Int(q.get()),
			},
		}
	}

	#[must_use]
	/// # Raw Quality Value.
	///
	/// This returns the raw — `NonZeroU8` — quality value. This method is
	/// only used internally by this crate.
	pub(crate) const fn raw(self) -> NonZeroU8 {
		match self {
			Self::Lossless(k) => k.max_encoder_quality(),
			Self::Lossy(_, q) => q,
		}
	}
}



#[derive(Debug, Clone, Copy)]
/// # Quality Value.
///
/// This holds a formatted quality value, which might be an integer, float, or
/// nothing (in the case of lossless).
///
/// It implements the `Display` trait, providing nice access for printing, etc.
pub enum QualityValue {
	/// # Float.
	///
	/// This is used by JPEG XL.
	Float(f32),

	/// # Integer.
	///
	/// This is used by AVIF and WebP, and also serves as a default for kinds
	/// that don't actually support encoding.
	Int(u8),

	/// # Lossless.
	///
	/// This is used by all three formats but has no specific value.
	Lossless,
}

impl fmt::Display for QualityValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Float(q) => write!(f, "{q:.1}"),
			Self::Int(q) => write!(f, "{q}"),
			Self::Lossless => f.write_str("lossless"),
		}
	}
}
