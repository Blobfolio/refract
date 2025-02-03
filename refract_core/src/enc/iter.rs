/*!
# `Refract` - Encoding Iterator.
*/

use crate::{
	FLAG_AVIF_RGB,
	FLAG_AVIF_ROUND_2,
	FLAG_NO_AVIF_YCBCR,
	FLAG_NO_LOSSLESS,
	FLAG_NO_LOSSY,
	FLAG_DID_LOSSLESS,
	ImageKind,
	Input,
	Output,
	PUBLIC_FLAGS,
	Quality,
	QualityRange,
	RefractError,
};
use std::{
	num::{
		NonZeroU8,
		NonZeroUsize,
	},
	time::{
		Duration,
		Instant,
	},
};



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
pub struct EncodeIter {
	/// # Source.
	src: Input,

	/// # Best Output.
	best: Output,

	/// # Current Output.
	candidate: Output,

	/// # Quality Stepper.
	steps: QualityRange,

	/// # Processing Time.
	time: Duration,

	/// # Flags.
	flags: u8,
}

/// ## Instantiation.
impl EncodeIter {
	/// # New.
	///
	/// Start a new iterator given a source and output format.
	///
	/// ## Errors
	///
	/// This will return an error if the output format does not support
	/// encoding.
	pub fn new(
		src: Input,
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
			src: match kind {
				// JPEG XL takes a compacted buffer.
				ImageKind::Jxl => src.into_native(),
				// Everybody else works from full RGBA.
				_ => src.into_rgba(),
			},
			best: Output::new(kind),
			candidate: Output::new(kind),

			steps: QualityRange::from(kind),
			time: Duration::from_secs(0),
			flags,
		})
	}
}

/// ## Getters.
impl EncodeIter {
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
impl EncodeIter {
	/// # Lossless Encoding.
	///
	/// Attempt to losslessly encode the image.
	///
	/// ## Errors
	///
	/// This will return an error if the encoder does not support lossless
	/// encoding, if there are errors during encoding, or if the resulting
	/// file offers no savings over the original.
	fn lossless(&mut self, flags: u8) -> Result<(), RefractError> {
		self.set_candidate_quality(None);

		let kind = self.output_kind();
		kind.encode_lossless(&self.src, &mut self.candidate, flags)?;

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

		let kind = self.output_kind();
		kind.encode_lossy(&self.src, &mut self.candidate, quality, flags)?;

		self.finish_candidate()
	}
}

/// ## Iteration Helpers.
impl EncodeIter {
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

	#[inline]
	/// # Discard Candidate.
	///
	/// Use this method to reject the last candidate because e.g. it looked
	/// terrible.
	///
	/// This will in turn raise the floor of the range so that the next
	/// iteration will test a higher quality.
	pub fn discard(&mut self) {
		self.steps.set_bottom(self.candidate.quality().raw());
	}

	/// # Keep Candidate.
	///
	/// Use this method to store the last candidate as the current best.
	///
	/// This will lower the ceiling of the range so that the next iteration
	/// will test a lower quality.
	pub fn keep(&mut self) {
		self.steps.set_top(self.candidate.quality().raw());
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
	/// `WebP` and `JPEG XL`, we need to test `AVIF` encoding in two rounds:
	/// once using full-range `RGB`, and once using limited-range `YCbCr`.
	fn next_avif(&mut self) -> Option<()> {
		let kind = self.output_kind();
		if kind != ImageKind::Avif { return None; }

		// The second round is a partial reboot, retrying encoding with
		// limited YCbCr range. If we haven't done that yet, let's do it
		// now!
		if 0 == self.flags & FLAG_AVIF_ROUND_2 {
			self.flags |= FLAG_AVIF_ROUND_2;

			// Reset the range and remove the RGB flag so that it can
			// start again (using the existing best as the size cap) in
			// limited-range mode.
			if 0 == self.flags & FLAG_NO_AVIF_YCBCR {
				self.steps.reboot(kind.min_encoder_quality(), kind.max_encoder_quality());
				self.flags &= ! FLAG_AVIF_RGB;

				// Recurse to pull the next result. If there isn't one, we're
				// done!
				if self.next_inner().is_some() { return Some(()); }
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
				self.steps.ignore(self.steps.top());
				if self.lossless(self.flags).is_ok() {
					self.keep_candidate();
				}
			}
		}

		// Okay, now lossy.
		if 0 == self.flags & FLAG_NO_LOSSY {
			let quality = self.steps.next()?;
			match self.lossy(quality, self.flags) {
				Ok(()) => Some(()),
				Err(RefractError::TooBig) => {
					// This was too big, so drop a step and see if the
					// next-next quality works out.
					self.steps.set_top_minus_one(quality);
					self.next_inner()
				},
				Err(_) => None,
			}
		}
		else { None }
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
}
