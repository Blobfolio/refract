/*!
# `Refract`: `WebP` Handling

This uses [`libwebp-sys2`](https://crates.io/crates/libwebp-sys2) bindings to Google's
`libwebp`. Operations should be equivalent to the corresponding `cwebp` output.
*/

use crate::{
	Input,
	Output,
	RefractError,
	traits::Encoder,
};
use libwebp_sys::{
	WEBP_MAX_DIMENSION,
	WebPConfig,
	WebPConfigInit,
	WebPConfigLosslessPreset,
	WebPEncode,
	WebPMemoryWrite,
	WebPMemoryWriter,
	WebPMemoryWriterClear,
	WebPMemoryWriterInit,
	WebPPicture,
	WebPPictureFree,
	WebPPictureImportRGBA,
	WebPPictureInit,
	WebPValidateConfig,
};
use std::num::NonZeroU8;

#[cfg(feature = "decode_ng")]
use crate::{
	ColorKind,
	traits::{
		Decoder,
		DecoderResult,
	},
};



/// # `WebP` Image.
pub(crate) struct ImageWebp;

#[cfg(feature = "decode_ng")]
impl Decoder for ImageWebp {
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
		let d = LibWebPDecode::try_from(raw)?;

		let width = usize::try_from(d.width).map_err(|_| RefractError::Overflow)?;
		let height = usize::try_from(d.height).map_err(|_| RefractError::Overflow)?;
		let size = width.checked_mul(height)
			.and_then(|x| x.checked_mul(4))
			.ok_or(RefractError::Overflow)?;

		let buf: Vec<u8> = unsafe { std::slice::from_raw_parts_mut(d.ptr, size) }
			.to_vec();

		if buf.len() == size {
			let color = ColorKind::from_rgba(&buf);
			Ok((buf, width, height, color))
		}
		else { Err(RefractError::Decode) }
	}
}

impl Encoder for ImageWebp {
	#[inline]
	/// # Encode Lossy.
	fn encode_lossy(input: &Input, output: &mut Output, quality: NonZeroU8, _flags: u8)
	-> Result<(), RefractError> {
		encode(input, output, Some(quality))
	}

	#[inline]
	/// # Encode Lossless.
	fn encode_lossless(input: &Input, output: &mut Output, _flags: u8)
	-> Result<(), RefractError> {
		encode(input, output, None)
	}
}



#[cfg(feature = "decode_ng")]
/// # Decode Wrapper.
///
/// This exists solely to help with garbage cleanup.
struct LibWebPDecode {
	width: i32,
	height: i32,
	ptr: *mut u8,
}

#[cfg(feature = "decode_ng")]
impl TryFrom<&[u8]> for LibWebPDecode {
	type Error = RefractError;
	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		use std::os::raw::c_int;
		use libwebp_sys::WebPDecodeRGBA;

		let mut width: c_int = 0;
		let mut height: c_int = 0;
		let result = unsafe {
			WebPDecodeRGBA(src.as_ptr(), src.len(), &mut width, &mut height)
		};

		if result.is_null() { Err(RefractError::Decode) }
		else {
			Ok(Self {
				width,
				height,
				ptr: result,
			})
		}
	}
}

#[cfg(feature = "decode_ng")]
impl Drop for LibWebPDecode {
	#[inline]
	fn drop(&mut self) { unsafe { libwebp_sys::WebPFree(self.ptr.cast()); } }
}



/// # Picture Wrapper.
///
/// This `C` struct is Rust-wrapped to help with garbage cleanup, but while
/// we're here, may as well provide initialization code too.
struct LibWebpPicture(WebPPicture);

impl TryFrom<&Input<'_>> for LibWebpPicture {
	type Error = RefractError;

	fn try_from(img: &Input) -> Result<Self, Self::Error> {
		// Check the source dimensions.
		let width = img.width_i32()?;
		let height = img.height_i32()?;
		if width > WEBP_MAX_DIMENSION || height > WEBP_MAX_DIMENSION {
			return Err(RefractError::Overflow);
		}

		// Set up the picture struct.
		let mut out = Self(unsafe { std::mem::zeroed() });
		maybe_die(unsafe { WebPPictureInit(&mut out.0) })?;

		out.0.use_argb = 1;
		out.0.width = width;
		out.0.height = height;
		out.0.argb_stride = width; // Stride always matches width for us.

		// Fill the pixel buffers.
		unsafe {
			let raw: &[u8] = &*img;
			maybe_die(WebPPictureImportRGBA(
				&mut out.0,
				raw.as_ptr().cast(), // This doesn't actually mutate.
				width << 2,
			))?;

			// A few additional sanity checks.
			let len = i32::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;
			let expected_size = (width * height) << 2;
			if expected_size == 0 || expected_size != len {
				return Err(RefractError::Encode);
			}
		}

		// A few more sanity checks.
		if out.0.use_argb != 1 || ! out.0.y.is_null() || out.0.argb.is_null() {
			return Err(RefractError::Encode);
		}

		Ok(out)
	}
}

impl Drop for LibWebpPicture {
	#[inline]
	fn drop(&mut self) { unsafe { WebPPictureFree(&mut self.0); } }
}



/// # Writer Wrapper.
///
/// This `C` struct is Rust-wrapped to help with garbage cleanup, but while
/// we're here, may as well provide initialization code too.
struct LibWebpWriter(*mut WebPMemoryWriter);

impl From<&mut WebPPicture> for LibWebpWriter {
	fn from(picture: &mut WebPPicture) -> Self {
		// A Writer wrapper function. (It has to be "safe".)
		extern "C" fn on_write(
			data: *const u8,
			data_size: usize,
			picture: *const WebPPicture,
		) -> std::os::raw::c_int {
			unsafe { WebPMemoryWrite(data, data_size, picture) }
		}

		// Hook in the writer.
		let writer = Self(unsafe {
			let mut writer: WebPMemoryWriter = std::mem::zeroed();
			WebPMemoryWriterInit(&mut writer);
			Box::into_raw(Box::new(writer))
		});

		picture.writer = Some(on_write);
		picture.custom_ptr = writer.0.cast::<std::ffi::c_void>();

		writer
	}
}

impl Drop for LibWebpWriter {
	#[inline]
	fn drop(&mut self) { unsafe { WebPMemoryWriterClear(self.0); } }
}



/// # Encode `WebP`.
///
/// This encodes a raw image source as a `WebP` using the provided
/// configuration profile, returning a regular byte vector of the result.
///
/// ## Errors
///
/// This will return an error if there are any problems along the way or if
/// the resulting image is empty (for some reason).
fn encode(
	img: &Input,
	candidate: &mut Output,
	quality: Option<NonZeroU8>,
) -> Result<(), RefractError> {
	// Setup.
	let config = make_config(quality)?;
	let mut picture = LibWebpPicture::try_from(img)?;
	let writer = LibWebpWriter::from(&mut picture.0);

	// Encode!
	maybe_die(unsafe { WebPEncode(&config, &mut picture.0) })?;

	// Copy output.
	let data = unsafe { Box::from_raw(writer.0) };
	candidate.set_slice(unsafe {
		std::slice::from_raw_parts_mut(data.mem, data.size)
	});

	// Clean-up.
	drop(picture);
	drop(writer);
	drop(data);

	Ok(())
}

/// # Make Config.
///
/// This generates an encoder configuration profile.
///
/// For lossy (with quality), this is roughly equivalent to:
///
/// ```bash
/// cwebp -m 6 -pass 10 -q {QUALITY}
/// ```
///
/// For lossless (no quality), this is instead like:
///
/// ```bash
/// cwebp -lossless -z 9 -q 100
/// ```
fn make_config(quality: Option<NonZeroU8>) -> Result<WebPConfig, RefractError> {
	let mut config: WebPConfig = unsafe { std::mem::zeroed() };
	maybe_die(unsafe { WebPConfigInit(&mut config) })?;
	maybe_die(unsafe { WebPValidateConfig(&config) })?;

	// Lossy bits.
	if let Some(quality) = quality {
		config.quality = f32::from(quality.get());
		config.method = 6;
		config.pass = 10;
	}
	// Lossless bits.
	else {
		maybe_die(unsafe { WebPConfigLosslessPreset(&mut config, 9) })?;
		config.lossless = 1;
		config.quality = 100.0;
	}

	Ok(config)
}

#[inline]
/// # Verify Encoder Status.
///
/// This converts unsuccessful AVIF system function results into proper Rust
/// errors.
const fn maybe_die(res: std::os::raw::c_int) -> Result<(), RefractError> {
	if 0 == res { Err(RefractError::Encode) }
	else { Ok(()) }
}
