/*!
# `Refract`: `AVIF` Handling

This program uses [`ravif`](https://crates.io/crates/ravif) for AVIF encoding.
It works very similarly to [`cavif`](https://crates.io/crates/cavif), but does
not support premultiplied/dirty alpha operations.
*/

use crate::{
	Image,
	Quality,
	RefractError,
	Refraction,
};
use fyi_msg::Msg;
use imgref::ImgVec;
use ravif::{
	ColorSpace,
	Config,
	RGBA8,
};
use std::{
	convert::TryFrom,
	ffi::OsStr,
	io::Write,
	num::{
		NonZeroU64,
		NonZeroU8,
	},
	os::unix::ffi::OsStrExt,
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # `AVIF`.
pub struct Avif {
	src: ImgVec<RGBA8>,
	src_size: NonZeroU64,

	dst: PathBuf,
	dst_size: Option<NonZeroU64>,
	dst_quality: Option<NonZeroU8>,

	tmp: PathBuf,
}

impl Avif {
	#[allow(trivial_casts)] // It is what it is.
	#[must_use]
	/// # New.
	///
	/// This instantiates a new instance from an [`Image`] struct. As
	/// [`Avif::find`] is the only other public-facing method, and as it is
	/// consuming, this is generally done as a single chained operation.
	pub fn new(src: &Image, img: ImgVec<RGBA8>) -> Self {
		let stub: &[u8] = unsafe { &*(src.path().as_os_str() as *const OsStr as *const [u8]) };

		Self {
			src: ravif::cleared_alpha(img),
			src_size: src.size(),

			dst: PathBuf::from(OsStr::from_bytes(&[stub, b".avif"].concat())),
			dst_size: None,
			dst_quality: None,

			tmp: PathBuf::from(OsStr::from_bytes(&[stub, b".PROPOSED.avif"].concat())),
		}
	}

	/// # Find the best!
	///
	/// This will generate lossy `AVIF` image copies in a loop with varying
	/// qualities, asking at each step whether or not the image looks OK. In
	/// most cases, an answer should be found in 5-10 steps.
	///
	/// If an acceptable `AVIF` candidate is found — based on user feedback and
	/// file size comparisons — it will be saved as `/path/to/SOURCE.avif`. For
	/// example, if the source lives at `/path/to/image.jpg`, the new version
	/// will live at `/path/to/image.jpg.avif`. In cases where the `AVIF` would
	/// be bigger than the source, no image is created.
	///
	/// Note: this method is consuming; the instance will not be usable
	/// afterward.
	///
	/// ## Errors
	///
	/// Returns an error if no acceptable `AVIF` can be found or if there are
	/// problems saving them.
	pub fn find(mut self) -> Result<Refraction, RefractError> {
		let prompt = Msg::custom(
			"AVIF",
			208,
			&format!(
				"Does \x1b[1;95m{}\x1b[0m look good?",
				self.tmp
					.file_name()
					.ok_or(RefractError::InvalidImage)?
					.to_string_lossy(),
			)
		);

		let mut quality = Quality::default();
		while let Some(q) = quality.next() {
			match self.make_lossy(q) {
				Ok(size) => {
					if prompt.prompt() {
						quality.set_max(q);

						// Move it to the destination.
						std::fs::rename(&self.tmp, &self.dst)
							.map_err(|_| RefractError::Write)?;

						// Update the details.
						self.dst_quality = Some(q);
						self.dst_size = Some(size);
					}
					else {
						quality.set_min(q);
					}
				},
				Err(RefractError::TooBig) => {
					if let Some(x) = NonZeroU8::new(q.get().saturating_sub(1)) {
						quality.set_max(x);
					}
					else { return Err(RefractError::NoAvif); }
				},
				Err(e) => {
					return Err(e);
				},
			}
		}

		// Clean up.
		if self.tmp.exists() {
			let _res = std::fs::remove_file(&self.tmp);
		}

		if let Some((size, quality)) = self.dst_size.zip(self.dst_quality) {
			Ok(Refraction::new(self.dst, size, quality))
		}
		else {
			// Get rid of the distribution file if it exists.
			if self.dst.exists() {
				let _res = std::fs::remove_file(self.dst);
			}

			Err(RefractError::NoAvif)
		}
	}

	/// # Make Lossy.
	///
	/// Generate an `AVIF` image at a given quality size.
	///
	/// ## Errors
	///
	/// This returns an error in cases where the resulting file size is larger
	/// than the source or previous best, or if there are any problems
	/// encountered during encoding or saving.
	fn make_lossy(&self, quality: NonZeroU8) -> Result<NonZeroU64, RefractError> {
		// Calculate qualities.
		let quality = quality.get();
		let alpha_quality = num_integer::div_floor(quality + 100, 2).min(
			quality + num_integer::div_floor(quality, 4) + 2
		);

		// Encode it!
		let (out, _, _) = ravif::encode_rgba(
			self.src.as_ref(),
			&Config {
	            quality,
	            speed: 1,
	            alpha_quality,
	            premultiplied_alpha: false,
	            color_space: ColorSpace::YCbCr,
	            threads: 0,
	        }
	    )
	    	.map_err(|_| RefractError::Write)?;

	    // What's the size?
	    let size = NonZeroU64::new(u64::try_from(out.len()).map_err(|_| RefractError::Write)?)
			.ok_or(RefractError::Write)?;

		// It has to be smaller than what we've already chosen.
		if let Some(dsize) = self.dst_size {
			if size >= dsize { return Err(RefractError::TooBig); }
		}
		// It has to be smaller than the source.
		else if size >= self.src_size {
			return Err(RefractError::TooBig);
		}

		// Write it to a file!
		std::fs::File::create(&self.tmp)
			.and_then(|mut file| file.write_all(&out).and_then(|_| file.flush()))
			.map_err(|_| RefractError::NoAvif)?;

		Ok(size)
	}
}
