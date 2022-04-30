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



/// # PNG Image.
pub(crate) struct ImagePng;

impl Decoder for ImagePng {
	#[allow(unsafe_code)]
	/// # Decode.
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
		// Grab the RGBA pixels, width, and height.
		let (mut raw, width, height): (Vec<u8>, usize, usize) = {
			// Parse the file.
			let decoder = spng::Decoder::new(raw)
				.with_output_format(spng::Format::Rgba8)
				.with_decode_flags(spng::DecodeFlags::TRANSPARENCY);
			let (info, mut reader) = decoder.read_info()
				.map_err(|_| RefractError::Decode)?;

			// Grab the dimensions.
			let width = usize::try_from(info.width)
				.map_err(|_| RefractError::Overflow)?;
			let height = usize::try_from(info.height)
				.map_err(|_| RefractError::Overflow)?;
			let size = width.checked_mul(height).and_then(|x| x.checked_mul(4))
				.ok_or(RefractError::Overflow)?;

			// Throw the pixels into a buffer.
			let mut out = Vec::new();
			out.try_reserve_exact(info.buffer_size).map_err(|_| RefractError::Overflow)?;
			unsafe { out.set_len(info.buffer_size); }
			reader.next_frame(&mut out)
				.map_err(|_| RefractError::Decode)?;

			// Make sure the buffer was actually filled to the right size.
			if out.len() != size {
				return Err(RefractError::Decode);
			}

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
