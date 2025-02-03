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
use std::{
	ffi::c_int,
	num::NonZeroU8,
};

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
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Decode.
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
		let d = LibWebPDecode::try_from(raw)?;
		if d.ptr.is_null() { return Err(RefractError::Decode); }

		let width = usize::try_from(d.width).map_err(|_| RefractError::Overflow)?;
		let height = usize::try_from(d.height).map_err(|_| RefractError::Overflow)?;
		let size = width.checked_mul(height)
			.and_then(|x| x.checked_mul(4))
			.ok_or(RefractError::Overflow)?;

		// Safety: the pointer is non-null; we have to trust libwebp gave us
		// the right dimensions.
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
	/// # Width.
	width: i32,

	/// # Height.
	height: i32,

	/// # Data Pointer.
	ptr: *mut u8,
}

#[cfg(feature = "decode_ng")]
impl TryFrom<&[u8]> for LibWebPDecode {
	type Error = RefractError;

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		use libwebp_sys::WebPDecodeRGBA;

		let mut width: c_int = 0;
		let mut height: c_int = 0;
		// Safety: this is an FFI call…
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
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libwebp handles deallocation.
		unsafe { libwebp_sys::WebPFree(self.ptr.cast()); }
	}
}



/// # Picture Wrapper.
///
/// This `C` struct is Rust-wrapped to help with garbage cleanup, but while
/// we're here, may as well provide initialization code too.
struct LibWebpPicture(WebPPicture);

impl TryFrom<&Input> for LibWebpPicture {
	type Error = RefractError;

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	fn try_from(img: &Input) -> Result<Self, Self::Error> {
		// Check the source dimensions.
		let width = img.width_i32()?;
		let height = img.height_i32()?;
		if width > WEBP_MAX_DIMENSION || height > WEBP_MAX_DIMENSION {
			return Err(RefractError::Overflow);
		}

		// Set up the picture struct.
		// Safety: libwebp expects zeroed memory.
		let mut out = Self(unsafe { std::mem::zeroed() });
		// Safety: this is an FFI call…
		maybe_die(unsafe { WebPPictureInit(&mut out.0) })?;

		out.0.use_argb = 1;
		out.0.width = width;
		out.0.height = height;
		out.0.argb_stride = width; // Stride always matches width for us.

		// Fill the pixel buffers.
		// Safety: this is an FFI call…
		unsafe {
			let raw: &[u8] = img;
			maybe_die(WebPPictureImportRGBA(
				&mut out.0,
				raw.as_ptr().cast(), // This doesn't actually mutate.
				width << 2,
			))?;

			// A few additional sanity checks.
			let len = i32::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;
			let expected_size = width * height * 4;
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
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libwebp handles deallocation.
		unsafe { WebPPictureFree(&mut self.0); }
	}
}



/// # Writer Wrapper.
///
/// This `C` struct is Rust-wrapped to help with garbage cleanup, but while
/// we're here, may as well provide initialization code too.
struct LibWebpWriter(*mut WebPMemoryWriter);

impl From<&mut WebPPicture> for LibWebpWriter {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	fn from(picture: &mut WebPPicture) -> Self {
		/// # A Writer Wrapper Function. (It has to be "safe".)
		extern "C" fn on_write(
			data: *const u8,
			data_size: usize,
			picture: *const WebPPicture,
		) -> c_int {
			// Safety: this is an FFI call…
			unsafe { WebPMemoryWrite(data, data_size, picture) }
		}

		// Hook in the writer.
		// Safety: this is an FFI call…
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
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libwebp handles deallocation.
		unsafe { WebPMemoryWriterClear(self.0); }
	}
}



#[expect(unsafe_code, reason = "Needed for FFI.")]
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
	// Safety: this is an FFI call…
	maybe_die(unsafe { WebPEncode(&config, &mut picture.0) })?;

	// Copy output.
	// Safety: we need to box the data to access it.
	let data = unsafe { Box::from_raw(writer.0) };
	// Safety: candidate makes a copy of the data so it's short lifetime is no
	// problem.
	candidate.set_slice(unsafe {
		std::slice::from_raw_parts_mut(data.mem, data.size)
	});

	// Clean-up.
	drop(picture);
	drop(writer);
	drop(data);

	Ok(())
}

#[expect(unsafe_code, reason = "Needed for FFI.")]
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
	// Safety: the subsequent call expects zeroed memory.
	let mut config: WebPConfig = unsafe { std::mem::zeroed() };
	// Safety: this is an FFI call…
	maybe_die(unsafe { WebPConfigInit(&mut config) })?;
	// Safety: this is an FFI call…
	maybe_die(unsafe { WebPValidateConfig(&config) })?;

	// Lossy bits.
	if let Some(quality) = quality {
		config.quality = f32::from(quality.get());
		config.method = 6;
		config.pass = 10;
	}
	// Lossless bits.
	else {
		// Safety: this is an FFI call…
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
const fn maybe_die(res: c_int) -> Result<(), RefractError> {
	if 0 == res { Err(RefractError::Encode) }
	else { Ok(()) }
}
