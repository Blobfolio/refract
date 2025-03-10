/*!
# `Refract`: `AVIF` Handling
*/

use crate::{
	ColorKind,
	FLAG_AVIF_RGB,
	Input,
	NZ_063,
	Output,
	RefractError,
	traits::{
		Decoder,
		DecoderResult,
		Encoder,
	},
};
use libavif_sys::{
	AVIF_CHROMA_DOWNSAMPLING_BEST_QUALITY,
	AVIF_CHROMA_SAMPLE_POSITION_COLOCATED,
	AVIF_CHROMA_UPSAMPLING_BILINEAR,
	AVIF_CODEC_CHOICE_AOM,
	AVIF_COLOR_PRIMARIES_BT709,
	AVIF_MATRIX_COEFFICIENTS_BT709,
	AVIF_MATRIX_COEFFICIENTS_IDENTITY,
	AVIF_PIXEL_FORMAT_YUV400,
	AVIF_PIXEL_FORMAT_YUV444,
	AVIF_RANGE_FULL,
	AVIF_RANGE_LIMITED,
	AVIF_RESULT_OK,
	AVIF_RGB_FORMAT_RGBA,
	AVIF_TRANSFER_CHARACTERISTICS_SRGB,
	avifDecoder,
	avifDecoderCreate,
	avifDecoderDestroy,
	avifDecoderReadMemory,
	avifEncoder,
	avifEncoderCreate,
	avifEncoderDestroy,
	avifEncoderWrite,
	avifImage,
	avifImageCreate,
	avifImageCreateEmpty,
	avifImageDestroy,
	avifImageRGBToYUV,
	avifImageYUVToRGB,
	avifResult,
	avifRGBImage,
	avifRGBImageAllocatePixels,
	avifRGBImageFreePixels,
	avifRGBImageSetDefaults,
	avifRWData,
	avifRWDataFree,
};
use std::num::NonZeroU8;



/// # AVIF Image.
pub(crate) struct ImageAvif;

impl Decoder for ImageAvif {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
		// Safety: these are FFI calls…
		let rgb = unsafe {
			// Decode the raw image to an avifImage.
			let image = LibAvifImage::empty()?;
			let decoder = LibAvifDecoder::new()?;
			if AVIF_RESULT_OK != avifDecoderReadMemory(
				decoder.0,
				image.0,
				raw.as_ptr(),
				raw.len(),
			) {
				return Err(RefractError::Decode);
			}

			// Turn the avifImage into an avifRGB.
			let mut rgb = LibAvifRGBImage::default();
			avifRGBImageSetDefaults(&mut rgb.0, image.0);
			rgb.0.format = AVIF_RGB_FORMAT_RGBA;
			rgb.0.depth = 8;
			if
				AVIF_RESULT_OK != avifRGBImageAllocatePixels(&mut rgb.0) ||
				AVIF_RESULT_OK != avifImageYUVToRGB(image.0, &mut rgb.0)
			{
				return Err(RefractError::Decode);
			}

			// Done!
			rgb
		};

		// Sanity check: the pixel buffer should exist?
		if rgb.0.pixels.is_null() { return Err(RefractError::Decode); }

		// Make sure the dimensions fit `usize`.
		let width = usize::try_from(rgb.0.width)
			.map_err(|_| RefractError::Overflow)?;

		let height = usize::try_from(rgb.0.height)
			.map_err(|_| RefractError::Overflow)?;

		let size = width.checked_mul(height)
			.and_then(|x| x.checked_mul(4))
			.ok_or(RefractError::Overflow)?;

		// Steal the buffer.
		// Safety: pixels are non-null, and each AVIF pixel is RGBA.
		let buf: Vec<u8> = unsafe {
			std::slice::from_raw_parts_mut(rgb.0.pixels, size)
		}.to_vec();

		// If it all checks out, return it!
		if buf.len() == size {
			let color = ColorKind::from_rgba(&buf);
			Ok((buf, width, height, color))
		}
		else { Err(RefractError::Decode) }
	}
}

impl Encoder for ImageAvif {
	/// # Maximum Quality.
	const MAX_QUALITY: NonZeroU8 = NZ_063;

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Encode Lossy.
	fn encode_lossy(img: &Input, candidate: &mut Output, quality: NonZeroU8, flags: u8)
	-> Result<(), RefractError> {
		let image = LibAvifImage::new(img, flags)?;
		let encoder = LibAvifEncoder::try_from(quality)?;

		// Encode!
		let mut data = LibAvifRwData(avifRWData::default());
		// Safety: this is an FFI call…
		maybe_die(unsafe { avifEncoderWrite(encoder.0, image.0, &mut data.0) })?;

		// But make sure it gave us something.
		if data.0.data.is_null() { return Err(RefractError::Encode); }

		// Grab the output.
		// Safety: the pointer is non-null; we have to trust libavif gave us
		// the correct size.
		candidate.set_slice(unsafe {
			std::slice::from_raw_parts(data.0.data, data.0.size)
		});

		drop(data);
		drop(encoder);
		drop(image);

		Ok(())
	}

	#[inline]
	/// # Encode Lossless.
	fn encode_lossless(input: &Input, output: &mut Output, flags: u8)
	-> Result<(), RefractError> {
		if input.is_greyscale() { Err(RefractError::NothingDoing) }
		else {
			Self::encode_lossy(input, output, Self::MAX_QUALITY, flags)
		}
	}
}



/// # AVIF Decoder.
///
/// This wraps the AVIF decoder. It exists solely for garbage cleanup.
struct LibAvifDecoder(*mut avifDecoder);

impl LibAvifDecoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # New.
	fn new() -> Result<Self, RefractError> {
		// Safety: this is an FFI call…
		let decoder = unsafe { avifDecoderCreate() };
		if decoder.is_null() {
			return Err(RefractError::Decode);
		}

		// Set up the threads.
		let threads = std::thread::available_parallelism().ok()
			.and_then(|n| i32::try_from(n.get()).ok())
			.unwrap_or(1)
			.max(1);

		// Safety: We're only holding a pointer; we need to dereference it to
		// update the values.
		unsafe {
			(*decoder).codecChoice = AVIF_CODEC_CHOICE_AOM;
			(*decoder).maxThreads = threads;
		}

		Ok(Self(decoder))
	}
}

impl Drop for LibAvifDecoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libavif handles deallocation.
		unsafe { avifDecoderDestroy(self.0); }
	}
}



/// # AVIF Encoder.
///
/// This wraps the AVIF encoder. It primarily exists to give us a way to free
/// resources on drop, but also handles setup.
struct LibAvifEncoder(*mut avifEncoder);

impl TryFrom<NonZeroU8> for LibAvifEncoder {
	type Error = RefractError;

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # New Instance.
	fn try_from(quality: NonZeroU8) -> Result<Self, RefractError> {
		// Convert quality to quantizers. AVIF is so convoluted...
		let (q, aq) = quality_to_quantizers(quality);

		// Total threads.
		let threads = std::thread::available_parallelism().ok()
			.and_then(|n| i32::try_from(n.get()).ok())
			.unwrap_or(1)
			.max(1);

		// Start up the encoder!
		// Safety: this is an FFI call…
		let encoder = unsafe { avifEncoderCreate() };
		if encoder.is_null() { return Err(RefractError::Encode); }

		// Safety: we're only holding a pointer; we need to dereference it to
		// update the member values.
		unsafe {
			(*encoder).codecChoice = AVIF_CODEC_CHOICE_AOM;
			(*encoder).maxThreads = threads;

			(*encoder).minQuantizer = i32::from(q);
			(*encoder).maxQuantizer = i32::from(q);

			(*encoder).minQuantizerAlpha = i32::from(aq);
			(*encoder).maxQuantizerAlpha = i32::from(aq);

			// There is a speed 0, but it is brutally slow and has very little
			// benefit.
			(*encoder).speed = 1;
		};

		Ok(Self(encoder))
	}
}

impl Drop for LibAvifEncoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libavif handles deallocation.
		unsafe { avifEncoderDestroy(self.0); }
	}
}



/// # Avif Image.
///
/// The struct includes initialization helpers, but exists primarily for
/// garbage cleanup.
struct LibAvifImage(*mut avifImage);

impl LibAvifImage {
	#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # New Instance.
	fn new(src: &Input, flags: u8) -> Result<Self, RefractError> {
		// Make sure dimensions fit u32.
		let width = src.width_u32();
		let height = src.height_u32();

		// AVIF dimensions can't exceed this amount. We might as well bail as
		// early as possible.
		if src.width() * src.height() > 16_384 * 16_384 {
			return Err(RefractError::Overflow);
		}

		let limited = 0 == flags & FLAG_AVIF_RGB;
		let greyscale: bool = src.is_greyscale();

		// Make an "avifRGBImage" from our buffer.
		let raw: &[u8] = src;
		let rgb = avifRGBImage {
			width,
			height,
			depth: 8,
			format: AVIF_RGB_FORMAT_RGBA,
			chromaUpsampling: AVIF_CHROMA_UPSAMPLING_BILINEAR,
			chromaDownsampling: AVIF_CHROMA_DOWNSAMPLING_BEST_QUALITY,
			avoidLibYUV: 0,
			ignoreAlpha: i32::from(! src.has_alpha()),
			alphaPremultiplied: 0,
			isFloat: 0,
			maxThreads: 1,
			pixels: raw.as_ptr().cast_mut(),
			rowBytes: width * 4,
		};

		// And convert it to YUV.
		// Safety: these are FFI calls…
		let yuv = unsafe {
			let tmp = avifImageCreate(
				width,
				height,
				8, // Depth.
				if greyscale { AVIF_PIXEL_FORMAT_YUV400 }
				else { AVIF_PIXEL_FORMAT_YUV444 }
			);

			// This shouldn't happen, but could, maybe.
			if tmp.is_null() { return Err(RefractError::Encode); }

			(*tmp).yuvRange =
				if limited { AVIF_RANGE_LIMITED }
				else { AVIF_RANGE_FULL };

			(*tmp).yuvChromaSamplePosition = AVIF_CHROMA_SAMPLE_POSITION_COLOCATED;
			(*tmp).colorPrimaries = AVIF_COLOR_PRIMARIES_BT709 as _;
			(*tmp).transferCharacteristics = AVIF_TRANSFER_CHARACTERISTICS_SRGB as _;
			(*tmp).matrixCoefficients =
				if greyscale || limited { AVIF_MATRIX_COEFFICIENTS_BT709 as _ }
				else { AVIF_MATRIX_COEFFICIENTS_IDENTITY as _ };

			maybe_die(avifImageRGBToYUV(tmp, &rgb))?;

			tmp
		};

		Ok(Self(yuv))
	}

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Empty.
	fn empty() -> Result<Self, RefractError> {
		// Safety: this is an FFI call…
		let image = unsafe { avifImageCreateEmpty() };
		if image.is_null() { Err(RefractError::Decode) }
		else { Ok(Self(image)) }
	}
}

impl Drop for LibAvifImage {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libavif handles deallocation.
		unsafe { avifImageDestroy(self.0); }
	}
}



#[derive(Default)]
/// # Avif RGB Image.
///
/// This struct exists only for garbage collection purposes. It is used for
/// decoding.
struct LibAvifRGBImage(avifRGBImage);

impl Drop for LibAvifRGBImage {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	fn drop(&mut self) {
		// Safety: libavif handles deallocation.
		unsafe { avifRGBImageFreePixels(&mut self.0); }
	}
}



/// # Data Struct.
///
/// This wrapper only exists to provide garbage cleanup.
struct LibAvifRwData(avifRWData);

impl Drop for LibAvifRwData {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libavif handles deallocation.
		unsafe { avifRWDataFree(&mut self.0); }
	}
}



#[inline]
/// # Verify Encoder Status.
///
/// This converts unsuccessful AVIF system function results into proper Rust
/// errors.
const fn maybe_die(res: avifResult) -> Result<(), RefractError> {
	if AVIF_RESULT_OK == res { Ok(()) }
	else { Err(RefractError::Encode) }
}

/// # Quality to Quantizer(s).
///
/// This converts the quality stepping from [`EncodeIter`] into appropriate
/// `libavif` quantizers.
///
/// The first step is to flip the provided value as [`EncodeIter`] and
/// `libavif` work backward relative to one another. (Or best is their worst.)
///
/// AVIF separates out color and alpha values. For the latter, we apply the
/// formula used by `ravif` as it seems to work well.
///
/// It should be noted that since we're starting from a `NonZeroU8`, we can't
/// actually test the worst possible AVIF quantizers. That's fine, though, as
/// they're never appropriate.
fn quality_to_quantizers(quality: NonZeroU8) -> (u8, u8) {
	// Color first.
	let q = 63 - quality.get().min(63);
	if q == 0 { return (0, 0); }

	// Alpha follows a neat little formula stolen from `ravif`. It is a lot
	// easier on the brain to recalibrate the value to be out of 100, then
	// re-recalibrate it to be out of 63.
	let aq = ratio_of(quality.get(), 63, 100);
	let aq = (aq + 100).wrapping_div(2)
		.min(aq + aq.wrapping_div(4) + 2);
	let aq = 63 - ratio_of(aq, 100, 63);

	(q, aq)
}

#[expect(clippy::cast_sign_loss, reason = "In and out are both unsigned.")]
#[expect(clippy::cast_possible_truncation, reason = "In and out are both `u8`.")]
#[inline]
/// # Ratio Of.
///
/// This simply takes a fraction, multiplies it against a new base, and returns
/// that value. It's a bit verbose, so is offloaded to its own place.
fn ratio_of(e: u8, d: u8, base: u8) -> u8 {
	(f32::from(e.min(d)) / f32::from(d) * f32::from(base))
		.clamp(0.0, f32::from(base)) as u8
}
