/*!
# `Refract`: The Hard Bits
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::module_name_repetitions)]



mod candidate;
mod encoder;
mod error;
mod image;
mod kind;
mod quality;
mod refraction;

pub use candidate::Candidate;
pub use encoder::Encoder;
pub use error::RefractError;
pub use image::Image;
pub use kind::ImageKind;
pub use quality::{
	MAX_QUALITY,
	MIN_QUALITY,
	Quality,
};
pub use refraction::Refraction;

use imgref::ImgVec;
use ravif::RGBA8;



/// # Load RGBA.
///
/// This is largely lifted from [`cavif`](https://crates.io/crates/cavif). It
/// is simplified slightly as we don't support premultiplied/dirty alpha.
pub(crate) fn load_rgba(mut data: &[u8]) -> Result<ImgVec<RGBA8>, RefractError> {
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
