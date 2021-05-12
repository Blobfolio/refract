/*!
# `Refract` - Encoding Iterator
*/

use ahash::RandomState;
use crate::{
	Candidate,
	FLAG_AVIF_RGB,
	FLAG_AVIF_ROUND_2,
	FLAG_AVIF_ROUND_3,
	FLAG_LOSSLESS,
	FLAG_NO_AVIF_LIMITED,
	Image,
	Output,
	OutputKind,
	RefractError,
	Source,
};
use std::{
	collections::HashSet,
	convert::TryFrom,
	num::{
		NonZeroU64,
		NonZeroU8,
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



#[derive(Debug, Clone)]
/// # Encoding Iterator.
///
/// This is a guided encoding "iterator" generated from [`Source::encode`].
///
/// If the encoder supports lossless encoding, that is attempted first, right
/// out the gate.
///
/// From there, [`EncodeIter::advance`] can be repeatedly called to produce
/// candidate images at various encoding qualities, returning a byte slice of
/// the last encoded image so you can e.g. write it to a file.
///
/// Each result should be expected for Quality Assurance before continuing,
/// with a call to either [`EncodeIter::keep`] if it looked good or
/// [`EncodeIter::discard`] if it sucked.
///
/// Feedback from keep/discard operations is factored into the iterator,
/// allowing it to adjust the min/max quality boundaries to avoid pointless
/// operations.
///
/// Once the iterator has finished, you can collect the total computation
/// duration by calling [`EncodeIter::time`] and/or call [`EncodeIter::take`]
/// to obtain the best candidate image discovered, if any.
pub struct EncodeIter<'a> {
	bottom: NonZeroU8,
	top: NonZeroU8,
	tried: HashSet<NonZeroU8, RandomState>,

	src: Image<'a>,
	size: NonZeroU64,
	kind: OutputKind,

	best: Option<Output>,
	candidate: Candidate,

	time: Duration,
	flags: u8,
}

impl<'a> EncodeIter<'a> {
	#[must_use]
	/// # New.
	///
	/// Start a new iterator with a given source and output format.
	pub(crate) fn new(src: &'a Source<'a>, kind: OutputKind, mut flags: u8) -> Self {
		let (bottom, top) = kind.quality_range();

		// Only AVIF requires special flags at the moment.
		if kind == OutputKind::Avif { flags |= FLAG_AVIF_RGB;  }
		else { flags = 0; }

		let mut out = Self {
			bottom,
			top,
			tried: HashSet::with_hasher(AHASH_STATE),

			src: match kind {
				// JPEG XL takes a compacted source.
				OutputKind::Jxl => src.img_compact(),
				// AVIF and WebP work from full buffers.
				OutputKind::Avif | OutputKind::Webp => src.img(),
			},
			size: src.size(),
			kind,

			best: None,
			candidate: Candidate::new(kind),

			time: Duration::from_secs(0),
			flags
		};

		// Try lossless compression.
		let now = Instant::now();
		out.tried.insert(out.kind.lossless_quality());
		if out.lossless().is_ok() {
			out.keep_candidate(true);
		}
		out.time += now.elapsed();

		// We're done!
		out
	}
}

/// ## Encoding.
impl EncodeIter<'_> {
	/// # Lossless encoding.
	///
	/// Attempt to losslessly encode the image.
	///
	/// ## Errors
	///
	/// This will return an error if the encoder does not support lossless
	/// encoding, if there are errors during encoding, or if the resulting
	/// file offers no savings over the original.
	fn lossless(&mut self) -> Result<(), RefractError> {
		self.candidate.set_quality(None);

		match self.kind {
			OutputKind::Avif => Err(RefractError::NoLossless),
			OutputKind::Jxl => super::jxl::make_lossless(&self.src, &mut self.candidate),
			OutputKind::Webp => super::webp::make_lossless(&self.src, &mut self.candidate),
		}?;

		self.candidate.verify(self.size)?;

		Ok(())
	}

	/// # Lossy encoding.
	///
	/// Attempt to lossily encode the image at the given quality setting.
	///
	/// The `main` parameter is used to differentiate between normal operations
	/// when `true` (i.e. [`EncodeIter::next`]) and the special final pass used
	/// by AVIF when `false`.
	///
	/// ## Errors
	///
	/// This bubbles up encoding-related errors, and will also return an error
	/// if the resulting file offers no savings over the current best.
	fn lossy(&mut self, quality: NonZeroU8, flags: u8) -> Result<(), RefractError> {
		self.candidate.set_quality(Some(quality));

		// No tiling is done as a final pass at the end; it only applies to
		// AVIF sessions.
		let normal = 0 == flags & FLAG_AVIF_ROUND_3;

		match self.kind {
			OutputKind::Avif => super::avif::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
				flags,
			),
			OutputKind::Jxl if normal => super::jxl::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
			),
			OutputKind::Webp if normal => super::webp::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
			),
			_ => Err(RefractError::NothingDoing),
		}?;

		self.candidate.verify(self.size)
	}
}

/// ## Iteration Helpers.
impl EncodeIter<'_> {
	/// # Crunch the Next Quality!
	///
	/// This method will attempt to re-encode the source using the next
	/// quality. If successful, it will return a byte slice of the raw image
	/// file.
	///
	/// Once all qualities have been tested, this will return `None`.
	pub fn advance(&mut self) -> Option<&[u8]> {
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
	/// Use this method to reject a given candidate because e.g. it didn't look
	/// good enough. This will in turn raise the floor of the range so that the
	/// next iteration will test a higher quality.
	pub fn discard(&mut self) {
		self.set_bottom(self.candidate.quality());
	}

	/// # Keep Candidate.
	///
	/// Use this method to store a given candidate as the current best. This
	/// will lower the ceiling of the range so that the next iteration will
	/// test a lower quality.
	pub fn keep(&mut self) {
		self.set_top(self.candidate.quality());
		self.keep_candidate(false);
	}

	/// # Keep Candidate.
	fn keep_candidate(&mut self, lossless: bool) {
		let mut flags = self.flags;
		if lossless { flags |= FLAG_LOSSLESS; }

		if self.candidate.is_verified() {
			// Replace the existing best.
			if let Some(ref mut output) = self.best {
				if output.update(&self.candidate).is_ok() {
					self.size = output.size();
					output.set_flags(flags);
				}
			}
			// Insert a first best.
			else if let Ok(mut output) = Output::try_from(&self.candidate) {
				output.set_flags(flags);
				self.size = output.size();
				self.best.replace(output);
			}
		}
	}

	/// # Next AVIF Round.
	///
	/// `AVIF` is complicated, even by next-gen image standards. Haha. Unlike
	/// `WebP` and `JPEG XL`, we need to test `AVIF` encoding in three rounds:
	/// once using full-range RGB, once using limited-range RGB, and one final
	/// (single) re-encoding of the best found with tiling disabled.
	fn next_avif(&mut self) -> Option<()> {
		if self.kind != OutputKind::Avif { return None; }

		// The second round is a partial reboot, retrying encoding with
		// limited YCbCr range. If we haven't done that yet, let's do it
		// now!
		if 0 == self.flags & FLAG_AVIF_ROUND_2 {
			self.flags |= FLAG_AVIF_ROUND_2;

			// Reset the range and remove the RGB flag so that it can
			// start again (using the existing best as the size cap) in
			// limited-range mode.
			if 0 == self.flags & FLAG_NO_AVIF_LIMITED {
				let (bottom, top) = self.kind.quality_range();
				self.bottom = bottom;
				self.top = top;
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

			if let Some(best) = self.best.as_ref() {
				let quality = best.quality();
				let flags = best.flags() | FLAG_AVIF_ROUND_3;
				self.lossy(quality, flags).ok()?;
				self.keep_candidate(false);
			}
		}

		None
	}

	#[inline]
	/// # (True) Next.
	///
	/// This is the actual worker method for [`EncodeIter::next`]. It is
	/// offloaded to a separate function to make it easier to track execution
	/// time.
	fn next_inner(&mut self) -> Option<()> {
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
	/// a value somewhere in the middle.
	fn next_quality(&mut self) -> Option<NonZeroU8> {
		debug_assert!(self.bottom <= self.top);

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
	/// Loewr the range's ceiling to the provided quality minus one because
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

/// ## Getters.
impl EncodeIter<'_> {
	#[must_use]
	/// # Candidate Slice.
	///
	/// Get the candidate as a slice.
	pub fn candidate(&self) -> Option<&[u8]> { self.candidate.as_slice().ok() }

	#[must_use]
	/// # Computation Time.
	///
	/// This method returns the total amount of time spent encoding the image,
	/// including lossless and lossy modes.
	///
	/// It makes for interesting data…
	pub const fn time(&self) -> Duration { self.time }

	/// # Take It.
	///
	/// Consume the iterator and return the best candidate found, if any. This
	/// should be called after iteration has finished, unless you don't
	/// actually care about the results.
	///
	/// ## Errors
	///
	/// This will return an error if no best candidate was found.
	pub fn take(self) -> Result<Output, RefractError> {
		self.best.ok_or(RefractError::NoBest(self.kind))
	}
}
