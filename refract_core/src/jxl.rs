/*!
# `Refract`: `JPEG XL` Handling

JPEG XL is still a work in progress. There don't seem to be any viewers — even
those provided by <https://gitlab.com/wg1/jpeg-xl>, ironically — that actually
work with all possible variations.

Makes testing a bitch.

Anyhoo, this lays the foundation for support.

Open issues:
 * `jpegxl-rs` has no greyscale support;
   * This is pretty easily fixed by passing the second parameter to `JxlColorEncodingSetToSRGB`.
 * `jpegxl-rs`-generated file sizes differ from `cjxl`-generated ones;
   * The `myrna.png` test image is a good example. With a distance of 1.9, it comes out to `199_379` vs `71_785` bytes. Clearly `cjxl` has some extra tricks, but what?
 * There are also random segfaults;

*/

use crate::RefractError;
use imgref::Img;
use jpegxl_rs::{
	encode::{
		ColorEncoding,
		EncoderFrame,
		EncoderResult,
		EncoderSpeed,
	},
	ThreadsRunner,
};
use rgb::RGBA8;
use std::{
	convert::TryFrom,
	num::NonZeroU8,
};



/// # Has Alpha?
///
/// Our reference images are all in RGBA format; this checks to see if the A
/// channel is actually being used.
fn has_alpha(img: Img<&[RGBA8]>) -> bool {
	img.pixels().any(|p| p.a != 255)
}

#[inline]
/// # Make Lossy.
///
/// Generate a lossy `JPEG XL` image at a given quality size.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossy(img: Img<&[RGBA8]>, quality: NonZeroU8) -> Result<Vec<u8>, RefractError> {
	// Map the quality to "distance", which is what the encoder actually uses.
	let f_quality = f32::from(150_u8 - quality.get()) / 10.0;
	encode(img, Some(f_quality))
}

#[inline]
/// # Make Lossy.
///
/// Generate a lossless `JPEG XL`.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossless(img: Img<&[RGBA8]>) -> Result<Vec<u8>, RefractError> {
	encode(img, None)
}

/// # Encode.
fn encode(img: Img<&[RGBA8]>, quality: Option<f32>) -> Result<Vec<u8>, RefractError> {
	// Make sure width and height fit the `u32` space.
	let width = u32::try_from(img.width()).map_err(|_| RefractError::Encode)?;
	let height = u32::try_from(img.height()).map_err(|_| RefractError::Encode)?;

	// Are we doing alpha?
	let alpha = has_alpha(img);

	// Convert the image pixels to a single byte slice, using 3 or 4 channels
	// depending on alphaness.
	let pixel_data: Vec<u8> =
		if alpha {
			use rgb::ComponentBytes;
			let (buf, _, _) = img.to_contiguous_buf();
			buf.as_bytes().to_vec()
		}
		else {
			// Drop the alpha channel data.
			img.pixels()
				.fold(
					Vec::with_capacity(img.width() * img.height() * 3),
					|mut acc, pix| {
						acc.extend_from_slice(&[pix.r, pix.g, pix.b]);
						acc
					}
				)
		};

	// Start an encoder.
	let runner = ThreadsRunner::default();
	let encoder = jpegxl_rs::encode::encoder_builder()
		.color_encoding(ColorEncoding::SRgb)
		.have_alpha(alpha)
		.lossless(quality.is_none())
		.parallel_runner(&runner)
		.quality(quality.unwrap_or(0.0))
		.speed(EncoderSpeed::Tortoise)
		.build()
		.map_err(|_| RefractError::Encode)?;

	// To support alpha, we have to use a frame.
	if alpha {
		encoder.encode_frame(&EncoderFrame::new(&pixel_data).num_channels(4), width, height)
			.map_err(|_| RefractError::Encode)
			.map(|x: EncoderResult<u8>| x.data.into())
	}
	// Otherwise we can just do it straight.
	else {
		encoder.encode(&pixel_data, width, height)
			.map_err(|_| RefractError::Encode)
			.map(|x: EncoderResult<u8>| x.data.into())
	}
}
