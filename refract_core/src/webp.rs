/*!
# `Refract`: `WebP` Handling

This uses [`libwebp-sys2`](https://crates.io/crates/libwebp-sys2) bindings to Google's
`libwebp`. Operations should be equivalent to the corresponding `cwebp` output.
*/

use crate::RefractError;
use imgref::Img;
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
use ravif::RGBA8;
use std::{
	convert::TryFrom,
	num::NonZeroU8,
};



#[inline]
/// # Make Lossy.
///
/// Generate a lossy `WebP` image at a given quality size.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossy(img: Img<&[RGBA8]>, quality: NonZeroU8) -> Result<Vec<u8>, RefractError> {
	encode(img, init_config(quality))
}

#[inline]
/// # Make Lossy.
///
/// Generate a lossless `WebP`. This is only useful for PNG sources.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossless(img: Img<&[RGBA8]>) -> Result<Vec<u8>, RefractError> {
	encode(img, init_lossless_config())
}

/// # Initialize `WebP` Lossy Configuration.
///
/// This generates an encoder configuration profile roughly equivalent to:
///
/// ```bash
/// cwebp -m 6 -pass 10 -q {QUALITY}
/// ```
fn init_config(quality: NonZeroU8) -> WebPConfig {
	let mut config: WebPConfig = unsafe { std::mem::zeroed() };
	unsafe {
		WebPConfigInit(&mut config);
		WebPValidateConfig(&config);
	};
	config.quality = f32::from(quality.get());
	config.method = 6;
	config.pass = 10;
	config
}

/// # Initialize `WebP` Lossless Configuration.
///
/// This generates an encoder configuration profile roughly equivalent to:
///
/// ```bash
/// cwebp -lossless -z 9 -q 100
/// ```
fn init_lossless_config() -> WebPConfig {
	let mut config: WebPConfig = unsafe { std::mem::zeroed() };
	unsafe {
		WebPConfigInit(&mut config);
		WebPValidateConfig(&config);
		WebPConfigLosslessPreset(&mut config, 9);
	}
	config.lossless = 1;
	config.quality = 100.0;
	config
}

/// # Initialize `WebP` Picture.
///
/// This converts the raw pixels into a `WebPPicture` object and writer,
/// required for later encoding.
///
/// ## Errors
///
/// This will return an error if there are problems along the way, including
/// invalid image dimensions or logical issues with the various components.
fn init_picture(source: Img<&[RGBA8]>) -> Result<(WebPPicture, *mut WebPMemoryWriter), RefractError> {
	use std::os::raw::c_int;

	// A Writer wrapper function. (It has to be "safe".)
	extern "C" fn on_write(
		data: *const u8,
		data_size: usize,
		picture: *const WebPPicture,
	) -> c_int {
		unsafe { WebPMemoryWrite(data, data_size, picture) }
	}

	// Check the source dimensions.
	let width = i32::try_from(source.width()).map_err(|_| RefractError::Encode)?;
	let height = i32::try_from(source.height()).map_err(|_| RefractError::Encode)?;
	if width > WEBP_MAX_DIMENSION || height > WEBP_MAX_DIMENSION {
		return Err(RefractError::Encode);
	}

	// Set up the picture struct.
	let mut picture: WebPPicture = unsafe { std::mem::zeroed() };
	if unsafe { WebPPictureInit(&mut picture) } == 0 {
		return Err(RefractError::Encode);
	}

	let argb_stride = i32::try_from(source.stride())
		.map_err(|_| RefractError::Encode)?;
	picture.use_argb = 1;
	picture.width = width;
	picture.height = height;
	picture.argb_stride = argb_stride;

	// Fill the pixel buffers.
	unsafe {
		let mut pixel_data = {
			use rgb::ComponentBytes;
			let (buf, _, _) = source.to_contiguous_buf();
			buf.as_bytes().to_vec()
		};
		let status = WebPPictureImportRGBA(
			&mut picture,
			pixel_data.as_mut_ptr(),
			argb_stride * 4,
		);

		// A few additional sanity checks.
		let expected_size = argb_stride * height * 4;
		if
			status == 0 ||
			expected_size == 0 ||
			i32::try_from(pixel_data.len()).unwrap_or(0) != expected_size
		{
			return Err(RefractError::Encode);
		}

		// Clean-up.
		std::mem::drop(pixel_data);
	}

	// A few more sanity checks.
	if picture.use_argb != 1 || ! picture.y.is_null() || picture.argb.is_null() {
		return Err(RefractError::Encode);
	}

	// Hook in the writer.
	let writer = unsafe {
		let mut writer: WebPMemoryWriter = std::mem::zeroed();
		WebPMemoryWriterInit(&mut writer);
		Box::into_raw(Box::new(writer))
	};

	picture.writer = Some(on_write);
	picture.custom_ptr = writer.cast::<std::ffi::c_void>();

	// Done!
	Ok((picture, writer))
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
fn encode(source: Img<&[RGBA8]>, config: WebPConfig) -> Result<Vec<u8>, RefractError> {
	let (mut picture, writer_ptr) = init_picture(source)?;
	if unsafe { WebPEncode(&config, &mut picture) } == 0 {
		return Err(RefractError::Encode);
	}

	// Copy output.
	let writer = unsafe { Box::from_raw(writer_ptr) };
	let output: Vec<u8> = unsafe {
		std::slice::from_raw_parts_mut(writer.mem, writer.size).to_vec()
	};

	// Clean-up.
	unsafe {
		WebPPictureFree(&mut picture);
		WebPMemoryWriterClear(writer_ptr);
		std::mem::drop(writer);
	}

	if output.is_empty() { Err(RefractError::Encode) }
	else { Ok(output) }
}
