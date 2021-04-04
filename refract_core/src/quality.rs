/*!
# `Refract`: Quality Range
*/

use std::num::NonZeroU8;



#[derive(Debug, Copy, Clone)]
/// # Quality Range.
pub struct Quality {
	min: NonZeroU8,
	max: NonZeroU8,
}

impl Default for Quality {
	#[inline]
	fn default() -> Self {
		Self {
			min: unsafe { NonZeroU8::new_unchecked(1) },
			max: unsafe { NonZeroU8::new_unchecked(100) },
		}
	}
}

impl Quality {
	#[must_use]
	/// # Next.
	pub fn next(self) -> Option<NonZeroU8> {
		if self.min == self.max { return None; }

		let max = self.max.get();
		let min = self.min.get();

		let mut diff = max - min;

		// Split the difference.
		if diff > 1 {
			diff = num_integer::div_floor(diff, 2);
		}

		Some(unsafe { NonZeroU8::new_unchecked(min + diff) })
	}

	/// # Cap Max.
	pub fn max(&mut self, quality: NonZeroU8) {
		self.max = quality;
		if self.max < self.min {
			self.min = self.max;
		}
	}

	/// # Cap Min.
	pub fn min(&mut self, quality: NonZeroU8) {
		self.min = quality;
		if self.max < self.min {
			self.max = self.min;
		}
	}
}
