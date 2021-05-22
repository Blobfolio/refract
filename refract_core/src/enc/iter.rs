/*!
# `Refract` - Encoding Iterator.
*/

use ahash::RandomState;
use crate::{
	FLAG_AVIF_RGB,
	FLAG_AVIF_ROUND_2,
	FLAG_AVIF_ROUND_3,
	FLAG_NO_AVIF_YCBCR,
	FLAG_NO_LOSSLESS,
	FLAG_NO_LOSSY,
	FLAG_DID_LOSSLESS,
	ImageKind,
	Input,
	Output,
	PUBLIC_FLAGS,
	Quality,
	RefractError,
};
use std::{
	collections::HashSet,
	num::{
		NonZeroU8,
		NonZeroUsize,
	},
	time::{
		Duration,
		Instant,
	},
};



/// # (Not) Random State.
///
/// Using a fixed seed value for `AHashSet` drops a few dependencies and
/// stops Valgrind from complaining about 64 lingering bytes from the runtime
/// static that would be used otherwise.
///
/// For our purposes, the variability of truly random keys isn't really needed.
const AHASH_STATE: RandomState = RandomState::with_seeds(13, 19, 23, 71);



#[derive(Debug)]
/// # Encoding Iterator.
///
/// This is a guided encoding "iterator" produced by providing an [`Input`]
/// source and an [`ImageKind`] output kind.
///
/// It attempts lossless and/or lossy encoding at varying qualities with each
/// call to [`EncodeIter::advance`], keeping track of the best candidate found
/// along the way, if any.
///
/// Each result should be inspected for Quality Assurance before continuing,
/// with a call to either [`EncodeIter::keep`] or [`EncodeIter::discard`] if it
/// looked good or bad respectively.
///
/// Feedback from [`EncodeIter::keep`] and [`EncodeIter::discard`] are factored
/// into each step, reducing the quality range to step over by roughly half
/// each time, avoiding pointless busy work.
///
/// Once iteration has finished, the computation time can be collected via
/// [`EncodeIter::time`] if you're interested, otherwise the instance can be
/// consumed, returning the "best" [`Output`] by calling [`EncodeIter::take`].
pub struct EncodeIter<'a> {
	bottom: NonZeroU8,
	top: NonZeroU8,
	tried: HashSet<NonZeroU8, RandomState>,

	src: Input<'a>,
	best: Output,
	candidate: Output,

	time: Duration,
	flags: u8,
}

/// ## Instantiation.
impl<'a> EncodeIter<'a> {
	/// # New.
	///
	/// Start a new iterator given a source and output format.
	///
	/// ## Errors
	///
	/// This will return an error if the output format does not support
	/// encoding.
	pub fn new(
		src: &'a Input<'a>,
		kind: ImageKind,
		mut flags: u8,
	) -> Result<Self, RefractError> {
		if ! kind.can_encode() {
			return Err(RefractError::ImageEncode(kind));
		}

		// Sanitize the flags.
		flags &= PUBLIC_FLAGS;
		if kind == ImageKind::Avif { flags |= FLAG_AVIF_RGB;  }
		else {
			// This only applies to AVIF.
			flags &= ! FLAG_NO_AVIF_YCBCR;
		}

		Ok(Self {
			bottom: kind.min_encoder_quality(),
			top: kind.max_encoder_quality(),
			tried: HashSet::with_hasher(AHASH_STATE),

			src: match kind {
				// JPEG XL takes a compacted buffer.
				ImageKind::Jxl => src.as_native(),
				// Everybody else works from full RGBA.
				_ => src.as_rgba(),
			},
			best: Output::new(kind),
			candidate: Output::new(kind),

			time: Duration::from_secs(0),
			flags
		})
	}
}

/// ## Getters.
impl EncodeIter<'_> {
	#[inline]
	#[must_use]
	/// # Candidate.
	///
	/// This returns a reference to the most recent candidate image, if any.
	pub const fn candidate(&self) -> Option<&Output> {
		if self.candidate.is_valid() { Some(&self.candidate) }
		else { None }
	}

	#[inline]
	#[must_use]
	/// # Input Kind.
	///
	/// This is a pass-through method for returning the kind of image used as
	/// the input source.
	pub const fn input_kind(&self) -> ImageKind { self.src.kind() }

	#[inline]
	#[must_use]
	/// # Input Size.
	///
	/// This is a pass-through method for returning the original file size of
	/// the source image.
	pub const fn input_size(&self) -> usize { self.src.size() }

	#[inline]
	#[must_use]
	/// # Output Kind.
	///
	/// This returns the output image format.
	pub const fn output_kind(&self) -> ImageKind { self.candidate.kind() }

	#[inline]
	#[must_use]
	/// # Output Size.
	///
	/// This returns the size of the current best output image, if any.
	pub fn output_size(&self) -> Option<NonZeroUsize> { self.best.size() }

	#[allow(clippy::missing_const_for_fn)] // Doesn't work.
	/// # Take the Best!
	///
	/// Consume the iterator and return the best candidate found, if any.
	///
	/// ## Errors
	///
	/// This will return an error if no best candidate was found.
	pub fn take(self) -> Result<Output, RefractError> {
		if self.best.is_valid() { Ok(self.best) }
		else { Err(RefractError::NoBest(self.output_kind())) }
	}

	/// # Target Size.
	///
	/// This returns the smaller of the input size and best size. Any time a
	/// new candidate is created, it must be smaller than these two or we'll
	/// just chuck it in the garbage.
	fn target_size(&self) -> usize {
		self.output_size()
			.map_or_else(|| self.input_size(), |s| s.get().min(self.input_size()))
	}

	#[inline]
	#[must_use]
	/// # Computation Time.
	///
	/// This method returns the total amount of time spent encoding the image,
	/// including lossless and lossy modes.
	///
	/// It makes for interesting data…
	pub const fn time(&self) -> Duration { self.time }
}

/// ## Encoding.
impl EncodeIter<'_> {
	/// # Lossless Encoding.
	///
	/// Attempt to losslessly encode the image.
	///
	/// ## Errors
	///
	/// This will return an error if the encoder does not support lossless
	/// encoding, if there are errors during encoding, or if the resulting
	/// file offers no savings over the original.
	fn lossless(&mut self) -> Result<(), RefractError> {
		self.set_candidate_quality(None);

		let kind = self.output_kind();
		match kind {
			ImageKind::Avif =>
				// Lossless compression isn't possible for greyscale images.
				if self.src.is_greyscale() {
					Err(RefractError::NothingDoing)
				}
				// Lossless AVIF encoding works exactly the same as lossy
				// encoding, it just uses maximum quality.
				else {
					super::avif::make_lossy(
						&self.src,
						&mut self.candidate,
						kind.max_encoder_quality(),
						self.flags,
					)
				},
			ImageKind::Jxl => super::jxl::make_lossless(&self.src, &mut self.candidate),
			ImageKind::Webp => super::webp::make_lossless(&self.src, &mut self.candidate),
			_ => Err(RefractError::NothingDoing),
		}?;

		self.finish_candidate()
	}

	/// # Lossy Encoding.
	///
	/// Attempt to lossily encode the image at the given quality setting.
	///
	/// ## Errors
	///
	/// This bubbles up encoding-related errors, and will also return an error
	/// if the resulting file offers no savings over the current best.
	fn lossy(&mut self, quality: NonZeroU8, flags: u8) -> Result<(), RefractError> {
		self.set_candidate_quality(Some(quality));

		// No tiling is done as a final pass at the end; it only applies to
		// AVIF sessions.
		let normal = 0 == flags & FLAG_AVIF_ROUND_3;

		match self.output_kind() {
			ImageKind::Avif => super::avif::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
				flags,
			),
			ImageKind::Jxl if normal => super::jxl::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
			),
			ImageKind::Webp if normal => super::webp::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
			),
			_ => Err(RefractError::NothingDoing),
		}?;

		self.finish_candidate()
	}
}

/// ## Iteration Helpers.
impl EncodeIter<'_> {
	/// # Crunch the Next Quality!
	///
	/// This is the tick method for the "iterator". Each call to it will
	/// work to produce a new candidate image at a new quality, returning a
	/// reference to it if successful, or `None` once it has finished.
	///
	/// Unlike standard Rust iterators, this is meant to take feedback between
	/// runs. See [`EncodeIter::discard`] and [`EncodeIter::keep`] for more
	/// information.
	pub fn advance(&mut self) -> Option<&Output> {
		// Start a timer.
		let now = Instant::now();

		// Handle the actual next business.
		let res = self.next_inner().or_else(|| self.next_avif());

		// Record the time spent.
		self.time += now.elapsed();

		// Return the result!
		if res.is_some() { self.candidate() }
		else { None }
	}

	/// # Discard Candidate.
	///
	/// Use this method to reject the last candidate because e.g. it looked
	/// terrible.
	///
	/// This will in turn raise the floor of the range so that the next
	/// iteration will test a higher quality.
	pub fn discard(&mut self) {
		self.set_bottom(self.candidate.quality().raw());
	}

	/// # Keep Candidate.
	///
	/// Use this method to store the last candidate as the current best.
	///
	/// This will lower the ceiling of the range so that the next iteration
	/// will test a lower quality.
	pub fn keep(&mut self) {
		self.set_top(self.candidate.quality().raw());
		self.keep_candidate();
	}

	#[inline]
	/// # Finish Writing Candidate.
	///
	/// This is a convenience method for validating a newly-generated
	/// candidate after lossy or lossless encoding.
	fn finish_candidate(&mut self) -> Result<(), RefractError> {
		self.candidate.finish(self.target_size())
	}

	/// # Keep Candidate.
	///
	/// This internal method does the actual keeping.
	fn keep_candidate(&mut self) {
		if self.candidate.is_valid() {
			self.candidate.copy_to(&mut self.best);
		}
	}

	/// # Next AVIF Round.
	///
	/// `AVIF` is complicated, even by next-gen image standards. Haha. Unlike
	/// `WebP` and `JPEG XL`, we need to test `AVIF` encoding in three rounds:
	/// once using full-range `RGB`, once using limited-range `YCbCr`, and one
	/// final (single) re-encoding of the best candidate with tiling disabled.
	fn next_avif(&mut self) -> Option<()> {
		if self.output_kind() != ImageKind::Avif { return None; }

		// The second round is a partial reboot, retrying encoding with
		// limited YCbCr range. If we haven't done that yet, let's do it
		// now!
		if 0 == self.flags & FLAG_AVIF_ROUND_2 {
			self.flags |= FLAG_AVIF_ROUND_2;

			// Reset the range and remove the RGB flag so that it can
			// start again (using the existing best as the size cap) in
			// limited-range mode.
			if 0 == self.flags & FLAG_NO_AVIF_YCBCR {
				let kind = self.output_kind();
				self.bottom = kind.min_encoder_quality();
				self.top = kind.max_encoder_quality();
				self.tried.clear();
				self.flags &= ! FLAG_AVIF_RGB;

				// Recurse to pull the next result.
				return self.next_inner();
			}
		}

		// The third round is a final one-off action: re-encode the "best"
		// (if any) using the same quality and mode, but with parallel
		// tiling disabled.
		//
		// If this pass manages to shrink the image some more, great, we'll
		// silently accept it; if not, no the previous best stays.
		if 0 == self.flags & FLAG_AVIF_ROUND_3 {
			self.flags |= FLAG_AVIF_ROUND_3;

			if self.best.is_valid() {
				let quality = self.best.quality().raw();
				let flags = self.best.flags() | FLAG_AVIF_ROUND_3;
				self.lossy(quality, flags).ok()?;
				self.keep_candidate();
			}
		}

		None
	}

	#[inline]
	/// # (True) Next.
	///
	/// This is the actual worker method for [`EncodeIter::advance`]. It is
	/// offloaded to a separate function to make it easier to track execution
	/// time.
	fn next_inner(&mut self) -> Option<()> {
		// Before we try lossy, we might lossless to do.
		if 0 == self.flags & FLAG_DID_LOSSLESS {
			self.flags |= FLAG_DID_LOSSLESS;
			if 0 == self.flags & FLAG_NO_LOSSLESS {
				self.tried.insert(self.top);
				if self.lossless().is_ok() {
					self.keep_candidate();
				}
			}
		}

		// Okay, now lossy.
		let quality = self.next_quality()?;
		match self.lossy(quality, self.flags) {
			Ok(_) => Some(()),
			Err(RefractError::TooBig) => {
				// Recurse to see if the next-next quality works out OK.
				self.set_top_minus_one(quality);
				self.next_inner()
			},
			Err(_) => None,
		}
	}

	/// # Next Quality.
	///
	/// This will choose an untested quality from the moving range, preferring
	/// a value somewhere in the middle of the boundaries.
	fn next_quality(&mut self) -> Option<NonZeroU8> {
		debug_assert!(self.bottom <= self.top);

		// Lossy encoding is disabled.
		if FLAG_NO_LOSSY == self.flags & FLAG_NO_LOSSY { return None; }

		let min = self.bottom.get();
		let max = self.top.get();
		let mut diff = max - min;

		// If the difference is greater than one, try a value near the middle.
		if diff > 1 {
			diff = num_integer::div_floor(diff, 2);
		}

		// See if this is new! We can't exceed u8::MAX here, so unsafe is fine.
		let next = unsafe { NonZeroU8::new_unchecked(min + diff) };
		if self.tried.insert(next) {
			return Some(next);
		}

		// If the above didn't work, let's see if there are any untested values
		// left and just run with the first.
		for i in min..=max {
			// Again, we can't exceed u8::MAX here, so unsafe is fine.
			let next = unsafe { NonZeroU8::new_unchecked(i) };
			if self.tried.insert(next) {
				return Some(next);
			}
		}

		// Looks like we're done!
		None
	}

	#[inline]
	/// # Set Candidate Quality.
	///
	/// This is a convenience method for populating the candidate quality for
	/// each new lossy or lossless run.
	fn set_candidate_quality(&mut self, quality: Option<NonZeroU8>) {
		self.candidate.set_quality(
			Quality::new(self.output_kind(), quality),
			self.flags,
		);
	}

	/// # Set Bottom.
	///
	/// Raise the range's floor because e.g. the image tested at this quality
	/// was not good enough (no point going lower).
	///
	/// This cannot go backwards or drop below the lower end of the range.
	/// Rather than panic, stupid values will be clamped accordingly.
	fn set_bottom(&mut self, quality: NonZeroU8) {
		self.bottom = quality
			.max(self.bottom)
			.min(self.top);
	}

	/// # Set Top.
	///
	/// Lower the range's ceiling because e.g. the image tested at this quality
	/// was fine (no point going higher).
	///
	/// This cannot go backwards or drop below the lower end of the range.
	/// Rather than panic, stupid values will be clamped accordingly.
	fn set_top(&mut self, quality: NonZeroU8) {
		self.top = quality
			.min(self.top)
			.max(self.bottom);
	}

	/// # Set Top Minus One.
	///
	/// Lower the range's ceiling to the provided quality minus one because
	/// e.g. the image tested at this quality came out too big.
	///
	/// The same could be achieved via [`EncodeIter::set_top`], but saves the
	/// wrapping maths.
	fn set_top_minus_one(&mut self, quality: NonZeroU8) {
		// We can't go lower than one. Short-circuit the process by setting
		// min and max to one. The iter will return `None` on the next run.
		if quality == unsafe { NonZeroU8::new_unchecked(1) } {
			self.top = self.bottom;
		}
		else {
			// We've already checked quality is bigger than one, so minus one
			// will fit just fine.
			self.set_top(unsafe { NonZeroU8::new_unchecked(quality.get() - 1) });
		}
	}
}
