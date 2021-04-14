/*!
# `Refract`: `AVIF` Handling

This program uses [`ravif`](https://crates.io/crates/ravif) for AVIF encoding.
It works very similarly to [`cavif`](https://crates.io/crates/cavif), but does
not support premultiplied/dirty alpha operations.
*/

use crate::RefractError;
use imgref::Img;
use ravif::{
	ColorSpace,
	Config,
	RGBA8,
};
use std::num::NonZeroU8;



/// # Make Lossy.
///
/// Generate an `AVIF` image at a given quality size.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossy(img: Img<&[RGBA8]>, quality: NonZeroU8) -> Result<Vec<u8>, RefractError> {
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
		.map_err(|_| RefractError::Encode)?;

	Ok(out)
}
