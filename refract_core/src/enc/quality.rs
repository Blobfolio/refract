/*!
# `Refract` - Encoding Quality.
*/

use crate::ImageKind;
use dactyl::NiceU8;
use std::{
	borrow::Cow,
	fmt,
	num::{
		NonZeroU8,
		NonZeroUsize,
	},
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
				// Avif is backwards.
				ImageKind::Avif => QualityValue::Int(63 - q.get()),
				// JPEG XL is backwards and fractional.
				ImageKind::Jxl => QualityValue::Float(f32::from(150_u8 - q.get()) / 10.0),
				// Webp is the only sane one. Haha.
				_ => QualityValue::Int(q.get()),
			},
		}
	}

	#[must_use]
	/// # Normalized Quality Value (Pre-Formatted).
	///
	/// This returns the quality value in the native format of the encoder. See
	/// [`QualityValueFmt`] for more information.
	pub fn quality_fmt(self) -> QualityValueFmt {
		match self {
			Self::Lossless(_) => QualityValueFmt::Lossless,
			Self::Lossy(k, q) => match k {
				// Avif is backwards.
				ImageKind::Avif => QualityValueFmt::Int(NiceU8::from(63 - q.get())),
				// JPEG XL is backwards and fractional.
				ImageKind::Jxl => QualityValueFmt::Float(NiceU8::from(150_u8 - q.get())),
				// Webp is the only sane one. Haha.
				_ => QualityValueFmt::Int(NiceU8::from(q)),
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
	/// This is used by AVIF and Webp, and also serves as a default for kinds
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



#[derive(Debug, Clone, Copy)]
/// # Quality Value (Format-Friendly).
///
/// This enum provies format-friendly representations of the different kinds of
/// quality values. (It exists primarily to reduce allocations.)
pub enum QualityValueFmt {
	/// # None.
	None,

	/// # Integer.
	///
	/// This is used by AVIF and Webp, and also serves as a default for kinds
	/// that don't actually support encoding.
	Int(NiceU8),

	/// # Lossless.
	///
	/// This is used by all three formats but has no specific value.
	Lossless,

	/// # Float.
	///
	/// This is used by JPEG XL. There is an implied precision of one; for
	/// print a period will have to be sneaked in ahead of the last digit.
	Float(NiceU8),
}

impl QualityValueFmt {
	#[must_use]
	/// # Is Empty?
	pub const fn is_empty(self) -> bool { matches!(self, Self::None) }

	#[must_use]
	/// # Length.
	pub const fn len(self) -> usize {
		match self {
			Self::None => 0,
			Self::Int(n) => n.len(),
			Self::Lossless => 8,
			Self::Float(n) =>
				if n.len() == 1 { 3 } // Add a zero and dot for print.
				else { n.len() + 1 }, // Add a dot for print.
		}
	}

	#[must_use]
	/// # As String.
	///
	/// Return a value suitable for print.
	///
	/// This requires allocation for `Self::Float` variants, but everything
	/// else can enjoy a cheap borrow.
	pub fn as_str(&self) -> Cow<str> {
		match self {
			Self::None => Cow::Borrowed(""),
			Self::Int(n) => Cow::Borrowed(n.as_str()),
			Self::Lossless => Cow::Borrowed("lossless"),

			// The floats require some touchup.
			Self::Float(n) => {
				// The length will never be zero, but let's prove it to the
				// compiler.
				let n = n.as_str();
				if let Some(len) = NonZeroUsize::new(n.len()) {
					// This can't fail, but the compiler won't know it.
					if let Some((mut a, b)) = n.split_at_checked(len.get() - 1) {
						// Make sure we have an integer in the first part.
						if a.is_empty() { a = "0" };

						let mut out = String::with_capacity(len.get() + 1);
						out.push_str(a);
						out.push('.'); // Divide by ten.
						out.push_str(b);
						return Cow::Owned(out);
					}
				}
				Cow::Borrowed("")
			},
		}
	}
}
