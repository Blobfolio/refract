/*!
# `Refract` - Quality Range.
*/

use crate::ImageKind;
use dactyl::NoHash;
use std::{
	collections::HashSet,
	num::NonZeroU8,
};



#[derive(Debug)]
/// # Quality Range.
pub struct QualityRange {
	bottom: NonZeroU8,
	top: NonZeroU8,
	tried: HashSet<NonZeroU8, NoHash>,
}

impl From<ImageKind> for QualityRange {
	#[inline]
	fn from(kind: ImageKind) -> Self {
		// We know these values are in the right order.
		Self {
			bottom: kind.min_encoder_quality(),
			top: kind.max_encoder_quality(),
			tried: HashSet::default(),
		}
	}
}

impl Iterator for QualityRange {
	type Item = NonZeroU8;

	#[allow(unsafe_code)]
	/// # Next Quality.
	///
	/// Return the next untested quality value from the moving range. In the
	/// early stages, the value will fall roughly in the middle of the ends,
	/// but as we run out of options, it may perform more sequentially.
	///
	/// Once every possibility (within the closing range) has been tried, `None`
	/// will be returned.
	fn next(&mut self) -> Option<Self::Item> {
		let min = self.bottom.get();
		let max = self.top.get();
		let mut diff = max - min;

		// If the difference is greater than one, cut it in half.
		if diff > 1 {
			diff = diff.wrapping_div(2);
		}

		// See if this is new!
		// Safety: we can't exceed u8::MAX here, so unsafe is fine.
		let next = unsafe { NonZeroU8::new_unchecked(min + diff) };
		if self.tried.insert(next) {
			return Some(next);
		}

		// If the above didn't work, let's see if there are any untested values
		// left and just run with the first.
		for i in min..=max {
			// Safety: again, we can't exceed u8::MAX here, so unsafe is fine.
			let next = unsafe { NonZeroU8::new_unchecked(i) };
			if self.tried.insert(next) {
				return Some(next);
			}
		}

		// Looks like we're done!
		None
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		// Log2 is a decent approximation of the number of guesses remaining.
		let diff = self.top.get() - self.bottom.get();
		if diff == 0 { (0, None) }
		else {
			let log2 = u8::BITS - diff.leading_zeros();
			(log2 as usize, None)
		}
	}
}

impl QualityRange {
	#[must_use]
	/// # New.
	///
	/// Create a new range between bottom and top, both inclusive.
	pub fn new(bottom: NonZeroU8, top: NonZeroU8) -> Self {
		if bottom <= top {
			Self {
				bottom,
				top,
				tried: HashSet::default(),
			}
		}
		// Reverse the order if needed.
		else {
			Self {
				bottom: top,
				top: bottom,
				tried: HashSet::default(),
			}
		}
	}

	/// # Reboot.
	///
	/// Recycle an instance by setting a new bottom and top (and clearing any
	/// history). The result is the same as calling [`QualityRange::new`], but
	/// potentially avoids reallocation.
	pub fn reboot(&mut self, mut bottom: NonZeroU8, mut top: NonZeroU8) {
		// Make sure they're in the right order.
		if bottom > top {
			std::mem::swap(&mut top, &mut bottom);
		}

		self.bottom = bottom;
		self.top = top;
		self.tried.clear();
	}
}

/// ## Getters.
impl QualityRange {
	#[inline]
	#[must_use]
	/// # Get the bottom.
	pub const fn bottom(&self) -> NonZeroU8 { self.bottom }

	#[inline]
	#[must_use]
	/// # Get the top.
	pub const fn top(&self) -> NonZeroU8 { self.top }
}

/// ## Setters.
impl QualityRange {
	#[inline]
	/// # Ignore Value.
	///
	/// Mark a value as having already been tried, preventing its appearance in
	/// the future.
	pub fn ignore(&mut self, quality: NonZeroU8) {
		self.tried.insert(quality);
	}

	#[inline]
	/// # Raise Bottom.
	///
	/// Raise the range's floor to this value (clamped to the existing bottom/
	/// top values).
	pub fn set_bottom(&mut self, bottom: NonZeroU8) {
		self.bottom = bottom.max(self.bottom).min(self.top);
	}

	#[inline]
	/// # Lower Top.
	///
	/// Lower the range's ceiling to this value (clamped to the existing
	/// bottom/top values).
	pub fn set_top(&mut self, top: NonZeroU8) {
		self.top = top.min(self.top).max(self.bottom);
	}

	#[allow(unsafe_code)]
	/// # Lower Top (Minus One).
	///
	/// This lowers the range's ceiling to the provided value minus one,
	/// avoiding wraps and overflows and whatnot.
	pub fn set_top_minus_one(&mut self, top: NonZeroU8) {
		// We can't go lower than one.
		if top == unsafe { NonZeroU8::new_unchecked(1) } {
			self.top = self.bottom;
		}
		else {
			self.set_top(unsafe { NonZeroU8::new_unchecked(top.get() - 1) });
		}
	}
}
