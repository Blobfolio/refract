/*!
# `Refract`: `WebP` Handling

This uses [`libwebp-sys2`](https://crates.io/crates/libwebp-sys2) bindings to Google's
`libwebp`. Operations should be equivalent to the corresponding `cwebp` output.
*/

use crate::{
	Output,
	Image,
	OutputKind,
	RefractError,
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
	convert::TryFrom,
	num::NonZeroU8,
};



/// # Picture Wrapper.
///
/// This exists solely to help with garbage cleanup, and, well, I suppose it
/// provides a place to handle the tedious initialization process. Haha.
struct TmpPicture(WebPPicture);

impl TryFrom<&Image<'_>> for TmpPicture {
	type Error = RefractError;

	fn try_from(img: &Image) -> Result<Self, Self::Error> {
		// Check the source dimensions.
		let width = i32::try_from(img.width()).map_err(|_| RefractError::Overflow)?;
		let height = i32::try_from(img.height()).map_err(|_| RefractError::Overflow)?;
		if width > WEBP_MAX_DIMENSION || height > WEBP_MAX_DIMENSION {
			return Err(RefractError::Overflow);
		}

		// Set up the picture struct.
		let mut out = Self(unsafe { std::mem::zeroed() });
		maybe_die(unsafe { WebPPictureInit(&mut out.0) })?;

		let argb_stride = i32::try_from(img.stride())
			.map_err(|_| RefractError::Encode)?;
		out.0.use_argb = 1;
		out.0.width = width;
		out.0.height = height;
		out.0.argb_stride = argb_stride;

		// Fill the pixel buffers.
		unsafe {
			let mut pixel_data = (&*img).to_vec();
			maybe_die(WebPPictureImportRGBA(
				&mut out.0,
				pixel_data.as_mut_ptr(),
				argb_stride * 4,
			))?;

			// A few additional sanity checks.
			let expected_size = argb_stride * height * 4;
			if
				expected_size == 0 ||
				i32::try_from(pixel_data.len()).unwrap_or(0) != expected_size
			{
				return Err(RefractError::Encode);
			}

			// Clean-up.
			std::mem::drop(pixel_data);
		}

		// A few more sanity checks.
		if out.0.use_argb != 1 || ! out.0.y.is_null() || out.0.argb.is_null() {
			return Err(RefractError::Encode);
		}

		Ok(out)
	}
}

impl Drop for TmpPicture {
	#[inline]
	fn drop(&mut self) { unsafe { WebPPictureFree(&mut self.0) } }
}



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
pub(super) fn make_lossy(img: &Image, quality: NonZeroU8) -> Result<Output, RefractError> {
	encode(img, Some(quality))
}

#[inline]
/// # Make Lossy.
///
/// Generate a lossless `WebP`. This is tested for all images, but will usually
/// only result in savings for PNG sources.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossless(img: &Image) -> Result<Output, RefractError> {
	encode(img, None)
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
fn encode(img: &Image, quality: Option<NonZeroU8>) -> Result<Output, RefractError> {
	// A Writer wrapper function. (It has to be "safe".)
	extern "C" fn on_write(
		data: *const u8,
		data_size: usize,
		picture: *const WebPPicture,
	) -> std::os::raw::c_int {
		unsafe { WebPMemoryWrite(data, data_size, picture) }
	}

	// Initialize configuration and quality.
	let config = quality.map_or_else(init_lossless_config, init_config)?;
	let mut picture = TmpPicture::try_from(img)?;

	// Hook in the writer.
	let writer = unsafe {
		let mut writer: WebPMemoryWriter = std::mem::zeroed();
		WebPMemoryWriterInit(&mut writer);
		Box::into_raw(Box::new(writer))
	};

	// Attach the writer to the picture.
	picture.0.writer = Some(on_write);
	picture.0.custom_ptr = writer.cast::<std::ffi::c_void>();

	// Encode!
	maybe_die(unsafe { WebPEncode(&config, &mut picture.0) })?;

	// Copy output.
	let data = unsafe { Box::from_raw(writer) };
	let raw: Box<[u8]> = unsafe {
		std::slice::from_raw_parts_mut(data.mem, data.size)
	}
		.to_vec()
		.into_boxed_slice();

	// Clean-up.
	drop(picture);
	unsafe {
		WebPMemoryWriterClear(writer);
		std::mem::drop(data);
	}

	// Send the output.
	Output::new(raw, quality.unwrap_or_else(|| OutputKind::Webp.lossless_quality()))
}

/// # Initialize `WebP` Lossy Configuration.
///
/// This generates an encoder configuration profile roughly equivalent to:
///
/// ```bash
/// cwebp -m 6 -pass 10 -q {QUALITY}
/// ```
fn init_config(quality: NonZeroU8) -> Result<WebPConfig, RefractError> {
	let mut config: WebPConfig = unsafe { std::mem::zeroed() };
	maybe_die(unsafe { WebPConfigInit(&mut config) })?;
	maybe_die(unsafe { WebPValidateConfig(&config) })?;
	config.quality = f32::from(quality.get());
	config.method = 6;
	config.pass = 10;
	Ok(config)
}

/// # Initialize `WebP` Lossless Configuration.
///
/// This generates an encoder configuration profile roughly equivalent to:
///
/// ```bash
/// cwebp -lossless -z 9 -q 100
/// ```
fn init_lossless_config() -> Result<WebPConfig, RefractError> {
	let mut config: WebPConfig = unsafe { std::mem::zeroed() };
	maybe_die(unsafe { WebPConfigInit(&mut config) })?;
	maybe_die(unsafe { WebPValidateConfig(&config) })?;
	maybe_die(unsafe { WebPConfigLosslessPreset(&mut config, 9) })?;
	config.lossless = 1;
	config.quality = 100.0;
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
