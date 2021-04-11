/*!
# `Refract`: Image
*/

use crate::{
	Candidate,
	Encoder,
	ImageKind,
	Quality,
	RefractError,
	MAX_QUALITY,
	Refraction,
};
use fyi_msg::Msg;
use imgref::ImgVec;
use ravif::RGBA8;
use std::{
	convert::TryFrom,
	num::{
		NonZeroU8,
		NonZeroU64,
	},
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # Image.
///
/// This holds data related to the source image, namely a reference to its
/// path, its size, its type, and all the pixels.
///
/// Guided encoding (for any given format) is done through [`Image::try_encode`].
pub struct Image<'a> {
	src: &'a PathBuf,
	img: ImgVec<RGBA8>,
	kind: ImageKind,
	size: NonZeroU64,
}

impl<'a> TryFrom<&'a PathBuf> for Image<'a> {
	type Error = RefractError;

	fn try_from(file: &'a PathBuf) -> Result<Self, Self::Error> {
		let raw = std::fs::read(file)
			.map_err(|_| RefractError::InvalidImage)?;

		Ok(Self {
			src: file,
			img: crate::load_rgba(&raw)?,
			kind: ImageKind::try_from(raw.as_slice())?,
			size: NonZeroU64::new(u64::try_from(raw.len()).map_err(|_| RefractError::InvalidImage)?)
				.ok_or(RefractError::InvalidImage)?,
		})
	}
}

impl<'a> Image<'a> {
	#[inline]
	/// # Try to (re)Encode!
	///
	/// This will attempt to find the smallest acceptable candidate image for
	/// the given encoder and source.
	///
	/// In general, this means testing lossy conversion at various qualities
	/// and prompting you to confirm whether or not the proposed image looks
	/// good.
	///
	/// For the combination of `PNG`/`WebP`, this will also attempt lossless
	/// conversion. This requires no prompt; if it is smaller it is kept as a
	/// starting point.
	///
	/// ## Errors
	///
	/// This method returns an error if no acceptable image is found, either
	/// because they all looked terrible or were larger than the source.
	pub fn try_encode(&self, enc: Encoder) -> Result<Refraction, RefractError> {
		enc.write_title();

		match enc {
			Encoder::Avif => {
				// We need to clean up the alpha data before processing AVIF.
				// We'll do this through a clone to avoid mutating the source
				// reference.
				let img = ravif::cleared_alpha(self.img.clone());
				let mut candidate = Candidate::new(self.src, img.as_ref(), enc);
				self.guided_encode(enc, &mut candidate)?;
				candidate.take_or(enc.error())
			},
			Encoder::Webp => {
				let mut candidate = Candidate::new(self.src, self.img.as_ref(), enc);

				// If the source is a PNG, let's go ahead and try lossless
				// conversion first. It is OK if this fails, but if it
				// succeeds, we'll use this as a starting point.
				if self.kind == ImageKind::Png {
					if let Ok(res) = enc.lossless(self.img.as_ref()) {
						if let Some(size) = self.normalize_size(res.len()) {
							if candidate.write_dst(&res).is_ok() {
								candidate.set_size_quality(size, MAX_QUALITY);
							}
						}
					}
				}

				self.guided_encode(enc, &mut candidate)?;
				candidate.take_or(enc.error())
			},
		}
	}

	/// # All Encodings Attack!
	///
	/// This is an internal helper method for [`Image::try_encode`] that
	/// actually performs all the lossy encoding operations.
	///
	/// This helps ensure the parent can perform some cleanup operations in the
	/// event of failure (while allow us to easily bubble errors).
	fn guided_encode(
		&self,
		enc: Encoder,
		candidate: &mut Candidate
	) -> Result<(), RefractError> {
		// The confirmation message we'll be presenting at each step.
		let prompt = Msg::plain(
			format!(
				"Does \x1b[1;95m{}\x1b[0m look good?",
				candidate.tmp_path()
					.file_name()
					.ok_or(RefractError::InvalidImage)?
					.to_string_lossy(),
			)
		)
			.with_indent(1);

		// The quality helper. Not an iterator, but almost.
		let mut quality = Quality::default();
		while let Some(q) = quality.next() {
			// Try to encode it. If this fails, there's an image problem so we
			// can bail.
			let data = enc.lossy(candidate.img(), q)?;

			// Check the size against the source and the current best. We only
			// need to ask how it looks if it is smaller.
			if let Some(size) = self.normalize_size(data.len()).filter(|&s| candidate.is_smaller(s)) {
				// Save it to a temporary file.
				candidate.write_tmp(&data)?;

				// Ask about it!

				// It looks fine, so we can set the ceiling to the level we
				// just tested.
				if prompt.prompt() {
					quality.set_max(q);
					candidate.keep(size, q)?;
				}
				// It looks bad, so we can set the floor to the level we just
				// tested.
				else {
					quality.set_min(q);
				}
			}
			// It was too big, so we know the ceiling is at least one lower
			// than what we just tested. Update accordingly and try again.
			else {
				let q = NonZeroU8::new(q.get().saturating_sub(1))
					.ok_or_else(|| enc.error())?;

				quality.set_max(q);
			}
		}

		Ok(())
	}

	/// # Normalize Size.
	///
	/// This converts a `usize` into a `NonZeroU64`, making sure it is smaller
	/// than the source size.
	///
	/// ## Errors
	///
	/// This returns an error if the new size is bigger or zero.
	fn normalize_size(&self, size: usize)
	-> Option<NonZeroU64> {
		let size = u64::try_from(size).ok()?;
		NonZeroU64::new(size).filter(|s| s < &self.size)
	}

	/// # Write Title.
	///
	/// This prints an ANSI-formatted title for when we begin working on the
	/// image.
	pub fn write_title(&self) {
		use std::io::Write;

		let path = self.src.to_string_lossy();
		let border = "-".repeat(path.len() + 2);

		let writer = std::io::stdout();
		let mut handle = writer.lock();
		let _res = handle.write_all(
			&[
				b"\x1b[38;5;199m+",
				border.as_bytes(),
				b"+\x1b[0m\n\x1b[38;5;199m| \x1b[0m",
				path.as_ref().as_bytes(),
				b"\x1b[38;5;199m |\n\x1b[38;5;199m+",
				border.as_bytes(),
				b"+\x1b[0m\n",
			].concat()
		).and_then(|_| handle.flush());
	}
}

impl<'a> Image<'a> {
	#[must_use]
	/// # Size.
	///
	/// Returns the disk size of the image (in bytes).
	pub const fn size(&self) -> NonZeroU64 { self.size }
}
