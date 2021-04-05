/*!
# `Refract`: Quality Range
*/

use std::num::NonZeroU8;



#[derive(Debug, Copy, Clone)]
/// # Quality Range.
///
/// This is a very simple range struct that allows our encodable types to drill
/// down to the perfect quality setting without having to test each and every
/// one.
///
/// See [`Quality::next`] for more information.
pub struct Quality {
	min: NonZeroU8,
	max: NonZeroU8,
	last: Option<NonZeroU8>,
}

impl Default for Quality {
	#[inline]
	fn default() -> Self {
		Self {
			min: unsafe { NonZeroU8::new_unchecked(1) },
			max: unsafe { NonZeroU8::new_unchecked(100) },
			last: None,
		}
	}
}

impl Quality {
	#[allow(clippy::should_implement_trait)] // It's fine.
	#[must_use]
	/// # Next Value.
	///
	/// This will return a value that sits roughly in the middle of the current
	/// min and max values, or `None` if we're out of options.
	///
	/// Combined with the mutable [`Quality::set_min`] and [`Quality::set_max`] capping
	/// methods that shrink the range, this allows us to find the "best" value
	/// in 5-10 steps instead of 100.
	///
	/// Think of it like a Bond villain room where the walls are closing in.
	pub fn next(&mut self) -> Option<NonZeroU8> {
		if self.min == self.max { return None; }

		let max = self.max.get();
		let min = self.min.get();

		let mut diff = max - min;

		// Split the difference.
		if diff > 1 {
			diff = num_integer::div_floor(diff, 2);
		}

		let next = unsafe { NonZeroU8::new_unchecked(min + diff) };
		if let Some(last) = self.last.replace(next) {
			if next == last { return None; }
		}

		Some(next)
	}

	/// # Cap Max.
	///
	/// Shrink the upper limit of the range, either because a tested value was
	/// fine or resulted in too big an image. In other words, use this when you
	/// know there's no point going any bigger.
	///
	/// If for some reason the passed value is lower than the current minimum,
	/// the floor will also be adjusted. In such cases, since floor and ceiling
	/// would then be equal, the next call to [`Quality::next`] will return
	/// `None`, ending the game.
	///
	/// ## Panics
	///
	/// This method will panic if the quality is greater than 100.
	pub fn set_max(&mut self, quality: NonZeroU8) {
		assert!(quality.get() <= 100);

		self.max = quality;
		if self.max < self.min {
			self.min = self.max;
		}
	}

	/// # Cap Min.
	///
	/// Shrink the lower limit of the range. This generally implies that a
	/// tested value was not good enough, hence there is no point testing an
	/// even lower value.
	///
	/// If for some reason the passed value is higher than the current maximum,
	/// the ceiling will also be adjusted. In such cases, since floor and ceiling
	/// would then be equal, the next call to [`Quality::next`] will return
	/// `None`, ending the game.
	///
	/// ## Panics
	///
	/// This method will panic if the quality is greater than 100.
	pub fn set_min(&mut self, quality: NonZeroU8) {
		assert!(quality.get() <= 100);

		self.min = quality;
		if self.max < self.min {
			self.max = self.min;
		}
	}
}
