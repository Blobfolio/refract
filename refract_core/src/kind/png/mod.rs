/*!
# `Refract` - PNG Images.
*/

mod alpha;

use crate::{
	ColorKind,
	RefractError,
	traits::{
		Decoder,
		DecoderResult,
	},
};
use lodepng::{
	Bitmap,
	RGBA,
};



/// # PNG Image.
pub(crate) struct ImagePng;

impl Decoder for ImagePng {
	/// # Decode.
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
		// Grab the RGBA pixels, width, and height.
		let (mut raw, width, height): (Vec<u8>, usize, usize) = {
			// Parse the file.
			let Bitmap::<RGBA> { buffer, width, height } = lodepng::decode32(raw)
				.map_err(|_| RefractError::Decode)?;

			// The pixel buffer should match the dimensions..
			let size = width.checked_mul(height).and_then(|x| x.checked_mul(4))
				.ok_or(RefractError::Overflow)?;

			// Throw the pixels into a buffer.
			let mut out = Vec::with_capacity(size);
			for RGBA { r, g, b, a } in buffer {
				out.push(r);
				out.push(g);
				out.push(b);
				out.push(a);
			}
			if out.len() != size { return Err(RefractError::Decode); }

			(out, width, height)
		};

		let color = ColorKind::from_rgba(&raw);

		// If we have alpha, let's take a quick detour to clean it up.
		if color.has_alpha() {
			alpha::clean_alpha(&mut raw, width, height);
		}

		Ok((raw, width, height, color))
	}
}
