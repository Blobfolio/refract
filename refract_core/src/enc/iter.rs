/*!
# `Refract` - Encoding Iterator
*/

use crate::{
	Candidate,
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
	tried: HashSet<NonZeroU8>,

	src: Image<'a>,
	size: NonZeroU64,
	kind: OutputKind,

	best: Option<Output>,
	candidate: Candidate,

	time: Duration,
	done: bool,
}

impl<'a> EncodeIter<'a> {
	#[must_use]
	/// # New.
	///
	/// Start a new iterator with a given source and output format.
	pub(crate) fn new(src: &'a Source<'a>, kind: OutputKind) -> Self {
		let (bottom, top) = kind.quality_range();

		let mut out = Self {
			bottom,
			top,
			tried: HashSet::new(),

			src: match kind {
				// JPEG XL takes a compacted source.
				OutputKind::Jxl => src.img_compact(),
				// AVIF and WebP both work from RGBA.
				OutputKind::Avif | OutputKind::Webp => src.img(),
			},
			size: src.size(),
			kind,

			best: None,
			candidate: Candidate::new(kind),

			time: Duration::from_secs(0),
			done: false,
		};

		// Try lossless compression.
		let now = Instant::now();
		out.tried.insert(out.kind.lossless_quality());
		if out.lossless().is_ok() {
			out.keep_candidate();
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

		self.candidate.verify(self.size)
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
	///
	/// When `main == false`, this will also return an error if the encoder
	/// does not require a final pass.
	fn lossy(&mut self, quality: NonZeroU8, main: bool) -> Result<(), RefractError> {
		self.candidate.set_quality(Some(quality));

		match self.kind {
			OutputKind::Avif => super::avif::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
				main,
			),
			OutputKind::Jxl if main => super::jxl::make_lossy(
				&self.src,
				&mut self.candidate,
				quality,
			),
			OutputKind::Webp if main => super::webp::make_lossy(
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
		let res = self.next_inner();

		// If we're done, see if it is worth doing one more (silent) pass
		// against the best found. This currently only applies to AVIF.
		if res.is_none() && ! self.done {
			let _res = self.next_final();
		}

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
		self.keep_candidate();
	}

	/// # Keep Candidate.
	fn keep_candidate(&mut self) {
		if self.candidate.is_verified() {
			// Replace the existing best.
			if let Some(ref mut output) = self.best {
				if output.update(&self.candidate).is_ok() {
					self.size = output.size();
				}
			}
			// Insert a first best.
			else if let Ok(output) = Output::try_from(&self.candidate) {
				self.size = output.size();
				self.best.replace(output);
			}
		}
	}

	#[inline]
	/// # (True) Next.
	///
	/// This is the actual worker method for [`EncodeIter::next`]. It is
	/// offloaded to a separate function to make it easier to track execution
	/// time.
	fn next_inner(&mut self) -> Option<()> {
		let quality = self.next_quality()?;
		match self.lossy(quality, true) {
			Ok(_) => Some(()),
			Err(RefractError::TooBig) => {
				// Recurse to see if the next-next quality works out OK.
				self.set_top_minus_one(quality);
				self.next_inner()
			},
			Err(_) => None,
		}
	}

	/// # One More Time.
	///
	/// This potentially takes one more run against the settings used for the
	/// discovered best candidate using stronger (slower) compression.
	///
	/// It is currently only used for AVIF images, as we cheat a little bit
	/// during iteration by splitting images up into multiple tiles for
	/// parallel processing. Tiling is great performance boost, but does often
	/// result in slightly larger files.
	///
	/// Anyhoo, for AVIFs, this will run once more without tiling and silently
	/// replace the best candidate if it winds up smaller.
	///
	/// ## Errors
	///
	/// This will return an erorr if there is no best candidate, no compression
	/// gains, etc., but the result is not actually used anywhere. If it works
	/// it is silently saved, if not, no changes occur.
	fn next_final(&mut self) -> Result<(), RefractError> {
		if self.done { return Ok(()); }
		self.done = true;

		let quality = self.best
			.as_ref()
			.map(Output::quality)
			.ok_or(RefractError::NothingDoing)?;

		self.lossy(quality, false)?;
		self.keep_candidate();

		Ok(())
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
