/*!
# `Refract` - Encoding
*/

use crate::{
	RefractError,
	Source,
	SourceKind,
};
use imgref::{
	Img,
	ImgExt,
};
use ravif::RGBA8;
use std::{
	borrow::Cow,
	collections::HashSet,
	convert::TryFrom,
	ffi::OsStr,
	fmt,
	num::{
		NonZeroU64,
		NonZeroU8,
	},
	os::unix::ffi::OsStrExt,
	path::Path,
};



/// # Minimum Encoding Quality
///
/// The minimum quality is 1.
pub const MIN_QUALITY: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(1) };

/// # Maximum Encoding Quality
///
/// The maximum quality is 100.
pub const MAX_QUALITY: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(100) };



#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Output Kind.
///
/// This is a list of supported encoders.
pub enum OutputKind {
	Avif,
	Webp,
}

impl fmt::Display for OutputKind {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl TryFrom<&[u8]> for OutputKind {
	type Error = RefractError;

	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		let kind = infer::get(src)
			.ok_or(RefractError::Encode)?;

		match kind.mime_type() {
			"image/avif" => Ok(Self::Avif),
			"image/webp" => Ok(Self::Webp),
			_ => Err(RefractError::Encode),
		}
	}
}

/// # Encoding.
impl OutputKind {
	/// # Encode Lossless.
	///
	/// Try to losslessly encode an image with the given encoder. If
	/// successful, the raw bytes are returned.
	///
	/// At the moment, this is only used for `PNG->WebP` conversions.
	///
	/// ## Errors
	///
	/// This will return an error if the encoder does not support lossless
	/// encoding, or if there are any other miscellaneous encoder issues along
	/// the way.
	pub fn lossless(self, img: Img<&[RGBA8]>) -> Result<Vec<u8>, RefractError> {
		let out = match self {
			Self::Avif => Err(RefractError::NoLossless),
			Self::Webp => crate::webp::make_lossless(img),
		}?;
		self.check_kind(out)
	}

	/// # Encode Lossy.
	///
	/// Try to lossily encode an image with the given encoder and quality
	/// setting. If successful, the raw bytes are returned.
	///
	/// ## Errors
	///
	/// If the encoder runs into trouble, an error will be returned.
	pub fn lossy(
		self,
		img: Img<&[RGBA8]>,
		quality: NonZeroU8
	) -> Result<Vec<u8>, RefractError> {
		let out = match self {
			Self::Avif => crate::avif::make_lossy(img, quality),
			Self::Webp => crate::webp::make_lossy(img, quality),
		}?;
		self.check_kind(out)
	}

	/// # Check Type.
	///
	/// This wlil double-check a given byte slice is the same kind as the
	/// encoder. The bytes will be passed through on success, otherwise an
	/// error is returned.
	fn check_kind(self, data: Vec<u8>) -> Result<Vec<u8>, RefractError> {
		let data_kind = Self::try_from(data.as_slice())?;
		if self == data_kind { Ok(data) }
		else { Err(RefractError::Encode) }
	}
}

/// # Getters.
impl OutputKind {
	#[must_use]
	/// # As Slice.
	///
	/// Return the encoder name as a byte slice.
	pub const fn as_bytes(self) -> &'static [u8] {
		match self {
			Self::Avif => b"AVIF",
			Self::Webp => b"WebP",
		}
	}

	#[must_use]
	/// # As Str.
	///
	/// Return the encoder name as a string slice.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Avif => "AVIF",
			Self::Webp => "WebP",
		}
	}

	#[must_use]
	/// # Extension as Slice.
	///
	/// Return the file extension for the encoder — with leading period — as a
	/// byte slice.
	pub const fn ext_bytes(self) -> &'static [u8] {
		match self {
			Self::Avif => b".avif",
			Self::Webp => b".webp",
		}
	}

	#[must_use]
	/// # Extension as Str.
	///
	/// Return the file extension for the encoder — with leading period — as a
	/// string slice.
	pub const fn ext_str(self) -> &'static str {
		match self {
			Self::Avif => ".avif",
			Self::Webp => ".webp",
		}
	}
}



#[derive(Debug, Clone)]
/// # Output Candidate.
///
/// This struct holds the information for an encoded image.
///
/// The raw data can be accessed using [`Output::data`] or written to a file
/// using [`Output::write`].
///
/// The format type, size, and encoder quality are retrievable via the
/// [`Output::kind`], [`Output::size`], and [`Output::quality`] methods,
/// respectively.
pub struct Output {
	data: Vec<u8>,
	kind: OutputKind,
	size: NonZeroU64,
	quality: NonZeroU8,
}

/// # Write Output.
impl Output {
	/// # Write to File.
	///
	/// Atomically write the image output to a file, overwriting the
	/// destination if it already exists.
	///
	/// This method does not enforce any particular naming conventions. You can
	/// use [`OutputKind::ext_str`]/[`OutputKind::ext_bytes`] to obtain the
	/// appropriate file extension for the type, or use [`Output::write_suffixed`]
	/// to append the extension for you (and write to said path).
	///
	/// ## Errors
	///
	/// This will return an error if there is no output to write, or the file
	/// system encounters problems along the way.
	pub fn write<P>(&self, path: P) -> Result<(), RefractError>
	where P: AsRef<Path> {
		use std::io::Write;

		tempfile_fast::Sponge::new_for(path.as_ref())
			.and_then(|mut out| out.write_all(&self.data).and_then(|_| out.commit()))
			.map_err(|_| RefractError::Write)
	}

	#[allow(trivial_casts)] // Triviality is necessary.
	/// # Write to File (Suffixed).
	///
	/// This method will append the appropriate file extension to the provided
	/// path, then write data using [`Output::write`].
	///
	/// ## Errors
	///
	/// This will return an error if there is no output to write, or the file
	/// system encounters problems along the way.
	pub fn write_suffixed<P>(&self, path: P) -> Result<(), RefractError>
	where P: AsRef<Path> {
		// It is a lot cheaper to work with bytes than any of the standard
		// library methods, besides which, they don't really provide a way to
		// append to a file name.
		self.write(OsStr::from_bytes(&[
			unsafe { &*(path.as_ref().as_os_str() as *const OsStr as *const [u8]) },
			self.kind.ext_bytes(),
		].concat()))
	}
}

/// # Getters.
impl Output {
	#[must_use]
	/// # Data.
	///
	/// Retrieve the raw image data as a byte slice.
	pub fn data(&self) -> &[u8] { &self.data }

	#[must_use]
	/// # Kind.
	///
	/// Return the encoder format used.
	pub const fn kind(&self) -> OutputKind { self.kind }

	#[must_use]
	/// # Quality.
	///
	/// Return the quality setting used to encode the image.
	///
	/// Note that a value of `100` typically indicates lossless compression, as
	/// it is very unlikely a lossy `100` would result in any image shrinking.
	/// At present, only `PNG->WebP` attempts lossless conversion.
	pub const fn quality(&self) -> NonZeroU8 { self.quality }

	#[must_use]
	/// # Size.
	///
	/// Return the size of the encoded image.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}



#[derive(Debug, Clone)]
/// # Guided Encoding.
///
/// This iterator is produced by calling [`Source::encode`]. Each tick
/// returns a re-encoded copy of the source image at a different quality level.
///
/// The caller should take a look at that output and call either [`OutputIter::keep`]
/// or [`OutputIter::discard`] prior to the next [`OutputIter::next`].
///
/// These methods will adjust the quality range accordingly — keep means the
/// image looked fine; discard means it didn't — allowing the smallest possible
/// candidate to be deduced in 5-10 steps instead of 100.
///
/// Once [`OutputIter::next`] returns `None`, you can call [`OutputIter::take`]
/// to consume the struct and return the best image found, if any.
///
/// See the `refract` CLI crate for example usage.
pub struct OutputIter<'a> {
	bottom: NonZeroU8,
	top: NonZeroU8,
	tried: HashSet<NonZeroU8>,

	src: Img<Cow<'a, [RGBA8]>>,
	src_size: NonZeroU64,
	src_kind: OutputKind,

	best: Option<Output>,
}

impl<'a> OutputIter<'a> {
	#[must_use]
	/// # New.
	///
	/// Start a new guided encoding iterator from a given source and encoder.
	pub fn new(src: &'a Source, kind: OutputKind) -> Self {
		match kind {
			OutputKind::Avif => {
				Self {
					bottom: MIN_QUALITY,
					top: MAX_QUALITY,
					tried: HashSet::new(),

					src: ravif::cleared_alpha(src.img_owned()).into(),
					src_size: src.size(),
					src_kind: kind,

					best: None,
				}
			},
			OutputKind::Webp => {
				let mut out = Self {
					bottom: MIN_QUALITY,
					top: MAX_QUALITY,
					tried: HashSet::new(),

					src: src.img().into(),
					src_size: src.size(),
					src_kind: kind,

					best: None,
				};

				// Try lossless conversion straight away. It is OK if this
				// fails, but if it succeeds, we'll use this as a starting
				// point.
				if src.kind() == SourceKind::Png {
					if let Ok(data) = kind.lossless(out.src.as_ref()) {
						if let Ok(size) = out.normalize_size(data.len()) {
							out.best = Some(Output {
								data,
								kind,
								size,
								quality: MAX_QUALITY,
							});
						}
					}
				}

				out
			},
		}
	}
}

impl<'a> Iterator for OutputIter<'a> {
	type Item = Output;

	fn next(&mut self) -> Option<Self::Item> {
		let quality = self.next_quality()?;
		let data = self.src_kind.lossy(self.src.as_ref(), quality).ok()?;

		match self.normalize_size(data.len()) {
			Ok(size) => Some(Output {
				data,
				kind: self.src_kind,
				size,
				quality,
			}),
			Err(RefractError::TooBig) => {
				self.set_top_minus_one(quality);
				self.next()
			},
			Err(_) => None,
		}
	}
}

/// # Iteration Helpers.
impl<'a> OutputIter<'a> {
	/// # Discard Candidate.
	///
	/// Use this method to reject a given candidate because e.g. it didn't look
	/// good enough. This will in turn raise the floor of the range so that the
	/// next iteration will test a higher quality.
	pub fn discard(&mut self, candidate: Output) {
		self.set_bottom(candidate.quality);
		drop(candidate);
	}

	/// # Keep Candidate.
	///
	/// Use this method to store a given candidate as the current best. This
	/// will lower the ceiling of the range so that the next iteration will
	/// test a lower quality.
	pub fn keep(&mut self, candidate: Output) {
		self.set_top(candidate.quality);
		self.best.replace(candidate);
	}

	/// # Next Quality.
	///
	/// This will choose an untested quality from the moving range, preferring
	/// a value somewhere in the middle.
	fn next_quality(&mut self) -> Option<NonZeroU8> {
		let min = self.bottom.get();
		let max = self.top.get();
		let mut diff = max - min;

		// If the difference is greater than one, try a value near the middle.
		if diff > 1 {
			diff = num_integer::div_floor(diff, 2);
		}

		// See if this is new!
		let next = unsafe { NonZeroU8::new_unchecked(min + diff) };
		if self.tried.insert(next) {
			return Some(next);
		}

		// If the above didn't work, let's see if there are any untested values
		// left and just run with the first.
		for i in min..=max {
			let next = unsafe { NonZeroU8::new_unchecked(i) };
			if self.tried.insert(next) {
				return Some(next);
			}
		}

		// Looks like we're done!
		None
	}

	/// # Normalize Size.
	///
	/// This converts a `usize` to a `NonZeroU64`, making sure it is smaller
	/// than the source and current best, if any.
	///
	/// ## Errors
	///
	/// This method will return an error if a `NonZeroU64` cannot be created or
	/// if the resulting value would be too big.
	fn normalize_size(&self, size: usize) -> Result<NonZeroU64, RefractError> {
		// The size has to fit in a `u64`.
		let size = u64::try_from(size).map_err(|_| RefractError::TooBig)?;

		// If we can't get a `NonZeroU64` from it, encoding has failed.
		let size = NonZeroU64::new(size).ok_or(RefractError::Encode)?;

		// It must be smaller than the current best.
		if let Some(s) = self.best_size() {
			if size >= s { return Err(RefractError::TooBig); }
		}
		// It must be smaller than the source. Since the best is going to be
		// smaller than the source, we only need to check this if there is no
		// best yet.
		else if size >= self.src_size { return Err(RefractError::TooBig); }

		Ok(size)
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
	/// The same could be achieved via [`OutputIter::set_top`], but saves the
	/// wrapping maths.
	fn set_top_minus_one(&mut self, quality: NonZeroU8) {
		// We can't go lower than one. Short-circuit the process by setting
		// min and max to one. The iter will return `None` on the next run.
		if quality == MIN_QUALITY {
			self.top = self.bottom;
		}
		else {
			self.set_top(unsafe { NonZeroU8::new_unchecked(quality.get() - 1) });
		}
	}
}

/// # Best Getters.
impl<'a> OutputIter<'a> {
	#[inline]
	#[must_use]
	/// # Best Size.
	///
	/// This returns the current best size, if any.
	pub fn best_size(&self) -> Option<NonZeroU64> {
		self.best.as_ref().map(|b| b.size)
	}

	#[inline]
	/// # Take.
	///
	/// Consume the iterator and return the best candidate found, if any.
	///
	/// ## Errors
	///
	/// If no candidates were found, an error is returned.
	pub fn take(self) -> Result<Output, RefractError> {
		self.best.ok_or(RefractError::Candidate(self.src_kind))
	}
}
