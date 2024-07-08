/*!
# `Refract` - JPEG Images.
*/

use crate::{
	ColorKind,
	RefractError,
	traits::{
		Decoder,
		DecoderResult,
	},
};
use jpeg_decoder::PixelFormat;
use rgb::{
	ComponentSlice,
	FromSlice,
};



/// # JPEG Image.
pub(crate) struct ImageJpeg;

impl Decoder for ImageJpeg {
	/// # Decode.
	fn decode(mut raw: &[u8]) -> Result<DecoderResult, RefractError> {
		// Decode the image.
		let mut jecoder = jpeg_decoder::Decoder::new(&mut raw);
		let pixels = jecoder.decode()
			.map_err(|_| RefractError::Decode)?;
		let info = jecoder.info().ok_or(RefractError::Decode)?;

		let width: usize = info.width.into();
		let height: usize = info.height.into();
		let size = width.checked_mul(height).and_then(|x| x.checked_mul(4))
			.ok_or(RefractError::Overflow)?;

		// So many ways to be a JPEG...
		let (raw, any_color): (Vec<u8>, bool) = match info.pixel_format {
			// Upscale greyscale to RGBA.
			PixelFormat::L8 => (
				pixels.iter()
					.fold(Vec::with_capacity(size), |mut acc, &px| {
						acc.extend_from_slice(&[px, px, px, 255]);
						acc
					}),
				false
			),
			// Upscale RGB to RGBA.
			PixelFormat::RGB24 =>  pixels.as_rgb()
				.iter()
				.map(|px| px.with_alpha(255))
				.fold(
					(Vec::with_capacity(size), false), |mut acc, px| {
					acc.0.extend_from_slice(px.as_slice());
					(
						acc.0,
						acc.1 || px.r != px.g || px.r != px.b,
					)
				}),
			// Lossless and CMYK aren't supported.
			PixelFormat::CMYK32 | PixelFormat::L16 => return Err(RefractError::Color),
		};

		// JPEGs don't have alpha.
		let color =
			if any_color { ColorKind::Rgb }
			else { ColorKind::Grey };

		// Make sure the buffer was actually filled to the right size.
		if raw.len() == size {
			Ok((raw, width, height, color))
		}
		else { Err(RefractError::Overflow) }
	}
}
