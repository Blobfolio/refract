/*!
# `Refract`: Image `AVIF`
*/

use crate::Image;
use crate::Quality;
use crate::RefractError;
use crate::Refraction;
use fyi_msg::Msg;
use imgref::ImgVec;
use ravif::ColorSpace;
use ravif::Config;
use ravif::Img;
use ravif::RGBA8;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::io::Write;
use std::num::NonZeroU64;
use std::num::NonZeroU8;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;



#[derive(Debug, Clone)]
/// # `AVIF`.
pub struct Avif<'a> {
	src: &'a [u8],

	dst: PathBuf,
	dst_size: Option<NonZeroU64>,
	dst_quality: Option<NonZeroU8>,

	tmp: PathBuf,
}

impl<'a> Avif<'a> {
	#[allow(trivial_casts)] // It is what it is.
	#[must_use]
	/// # New.
	///
	/// ## Errors
	///
	/// This returns an error if the image cannot be read.
	pub fn new(src: &'a Image) -> Self {
		let stub: &[u8] = unsafe { &*(src.path().as_os_str() as *const OsStr as *const [u8]) };

		Self {
			src: src.raw(),

			dst: PathBuf::from(OsStr::from_bytes(&[stub, b".avif"].concat())),
			dst_size: None,
			dst_quality: None,

			tmp: PathBuf::from(OsStr::from_bytes(&[stub, b".PROPOSED.avif"].concat())),
		}
	}

	/// # Find the best!
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

		// Convert the image to a pixel buffer.
		let mut img = load_rgba(self.src)?;
		img = ravif::cleared_alpha(img);

		let mut quality = Quality::default();
		while let Some(q) = quality.next() {
			match self.make_lossy(img.as_ref(), q) {
				Ok(size) => {
					if prompt.prompt() {
						quality.max(q);

						// Move it to the destination.
						std::fs::rename(&self.tmp, &self.dst)
							.map_err(|_| RefractError::Write)?;

						// Update the details.
						self.dst_quality = Some(q);
						self.dst_size = Some(size);
					}
					else {
						quality.min(q);
					}
				},
				Err(RefractError::TooBig) => {
					if let Some(x) = NonZeroU8::new(q.get().saturating_sub(1)) {
						quality.max(x);
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
	fn make_lossy(&self, img: Img<&[RGBA8]>, quality: NonZeroU8) -> Result<NonZeroU64, RefractError> {
		// Calculate qualities.
		let quality = quality.get();
		let alpha_quality = num_integer::div_floor(quality + 100, 2).min(
			quality + num_integer::div_floor(quality, 4) + 2
		);

		// Encode it!
		let (out, _, _) = ravif::encode_rgba(
			img,
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
		else if size.get() >= self.src.len() as u64 {
			return Err(RefractError::TooBig);
		}

		// Write it to a file!
		std::fs::File::create(&self.tmp)
			.and_then(|mut file| file.write_all(&out).and_then(|_| file.flush()))
			.map_err(|_| RefractError::NoAvif)?;

		Ok(size)
	}
}

/// # Load RGBA.
///
/// This is largely lifted from [`cavif`](https://crates.io/crates/cavif). It
/// is simplified slightly as we don't support premultiplied/dirty alpha.
fn load_rgba(mut data: &[u8]) -> Result<ImgVec<RGBA8>, RefractError> {
	use rgb::FromSlice;

	// PNG.
	if data.get(0..4) == Some(&[0x89,b'P',b'N',b'G']) {
		let img = lodepng::decode32(data)
			.map_err(|_| RefractError::InvalidImage)?;

		Ok(ImgVec::new(img.buffer, img.width, img.height))
	}
	// JPEG.
	else {
		use jpeg_decoder::PixelFormat::{CMYK32, L8, RGB24};

		let mut jecoder = jpeg_decoder::Decoder::new(&mut data);
		let pixels = jecoder.decode()
			.map_err(|_| RefractError::InvalidImage)?;
		let info = jecoder.info().ok_or(RefractError::InvalidImage)?;

		// So many ways to be a JPEG...
		let buf: Vec<_> = match info.pixel_format {
			// Upscale greyscale to RGBA.
			L8 => {
				pixels.iter().copied().map(|g| RGBA8::new(g, g, g, 255)).collect()
			},
			// Upscale RGB to RGBA.
			RGB24 => {
				let rgb = pixels.as_rgb();
				rgb.iter().map(|p| p.alpha(255)).collect()
			},
			// CMYK doesn't work.
			CMYK32 => return Err(RefractError::InvalidImage),
		};

		Ok(ImgVec::new(buf, info.width.into(), info.height.into()))
	}
}
