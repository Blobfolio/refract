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
	path::{
		Path,
		PathBuf,
	},
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
		match enc {
			Encoder::Avif => {
				// We need to clean up the alpha data before processing AVIF.
				// We'll do this through a clone to avoid mutating the source
				// reference.
				let img = ravif::cleared_alpha(self.img.clone());
				let mut candidate = Candidate::new(self.src, img.as_ref(), enc);

				let res = self.guided_encode(enc, &mut candidate);

				// Clean up if we can.
				let _res = candidate.clean();

				// If the guided encode failed, return that error.
				if let Err(e) = res { return Err(e); }

				// Return the answer!
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

				let res = self.guided_encode(enc, &mut candidate);

				// Clean up if we can.
				let _res = candidate.clean();

				// If the guided encode failed, return that error.
				if let Err(e) = res { return Err(e); }

				// Return the answer!
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
		let prompt = make_prompt(enc.name(), candidate.tmp_path())?;

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
}

impl<'a> Image<'a> {
	#[must_use]
	/// # Size.
	///
	/// Returns the disk size of the image (in bytes).
	pub const fn size(&self) -> NonZeroU64 { self.size }
}



/// # Get Prompt.
///
/// This returns a [`Msg`] that is will be printed to the screen, asking if the
/// proposed image looks good.
///
/// ## Errors
///
/// This returns an error if the filename cannot be represented as a string.
fn make_prompt(name: &str, path: &Path) -> Result<Msg, RefractError> {
	Ok(Msg::custom(
		name,
		208,
		&format!(
			"Does \x1b[1;95m{}\x1b[0m look good?",
			path.file_name()
				.ok_or(RefractError::InvalidImage)?
				.to_string_lossy(),
		)
	))
}
