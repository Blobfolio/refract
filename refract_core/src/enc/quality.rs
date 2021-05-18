/*!
# `Refract` - Encoded Candidate
*/

use crate::OutputKind;
use std::{
	fmt,
	num::NonZeroU8,
};



#[derive(Debug, Clone, Copy)]
/// # Encoding Quality.
///
/// This is a simple enum used to present encoder quality settings in a
/// consistent way.
pub enum Quality {
	/// # Lossless.
	Lossless(OutputKind),
	/// # Lossy.
	Lossy(OutputKind, NonZeroU8),
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
	#[allow(clippy::option_if_let_else)] // Doesn't work with const.
	#[must_use]
	/// # New.
	pub const fn new(kind: OutputKind, quality: Option<NonZeroU8>) -> Self {
		if let Some(q) = quality {
			Self::Lossy(kind, q)
		}
		else {
			Self::Lossless(kind)
		}
	}
}

/// ## Getters.
impl Quality {
	#[must_use]
	/// # Kind.
	pub const fn kind(self) -> OutputKind {
		match self {
			Self::Lossless(k) | Self::Lossy(k, _) => k,
		}
	}

	#[must_use]
	/// # Label.
	///
	/// This is what to call the quality. (AVIF uses the fancy word "quantizer".)
	pub const fn label(self) -> &'static str {
		match self.kind() {
			OutputKind::Avif => "quantizer",
			_ => "quality",
		}
	}

	#[must_use]
	/// # Normalized Quality Value.
	///
	/// This returns the quality value in the native format of the encoder.
	pub fn quality(self) -> QualityValue {
		match self {
			Self::Lossless(_) => QualityValue::Lossless,
			Self::Lossy(k, q) => match k {
				OutputKind::Avif => QualityValue::Int(63 - q.get()),
				OutputKind::Jxl => QualityValue::Float(f32::from(150_u8 - q.get()) / 10.0),
				OutputKind::Webp => QualityValue::Int(q.get()),
			}
		}
	}

	#[must_use]
	/// # Raw Quality Value.
	///
	/// This is not particularly useful outside the crate, but provided just in
	/// case.
	pub const fn raw(self) -> NonZeroU8 {
		match self {
			Self::Lossless(k) => k.lossless_quality(),
			Self::Lossy(_, q) => q,
		}
	}
}



#[derive(Debug, Clone, Copy)]
/// # Quality Value.
///
/// This holds a formatted quality value, necessary because every encoder does
/// shit its own way.
pub enum QualityValue {
	/// # Float.
	///
	/// This is used by JPEG XL.
	Float(f32),

	/// # Integer.
	///
	/// This is used by AVIF and WebP.
	Int(u8),

	/// # Lossless.
	///
	/// This is used by all three formats but has no specific value.
	Lossless,
}

impl fmt::Display for QualityValue {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Float(q) => write!(f, "{:.1}", q),
			Self::Int(q) => write!(f, "{}", q),
			Self::Lossless => f.write_str("lossless"),
		}
	}
}
