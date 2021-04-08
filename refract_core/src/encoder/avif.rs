/*!
# `Refract`: `AVIF` Handling

This program uses [`ravif`](https://crates.io/crates/ravif) for AVIF encoding.
It works very similarly to [`cavif`](https://crates.io/crates/cavif), but does
not support premultiplied/dirty alpha operations.
*/

use crate::{
	Image,
	RefractError,
};
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

	crate::impl_find!("AVIF", RefractError::NoAvif);

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
	    let size = NonZeroU64::new(u64::try_from(out.len()).map_err(|_| RefractError::TooBig)?)
			.ok_or(RefractError::TooBig)?;

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
			.map_err(|_| RefractError::Write)?;

		Ok(size)
	}
}
