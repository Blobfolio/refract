/*!
# `Refract`: `AVIF` Handling
*/

use crate::{
	RefractError,
	TreatedSource,
};
use libavif_sys::{
	AVIF_CHROMA_UPSAMPLING_BILINEAR,
	AVIF_CODEC_CHOICE_RAV1E,
	AVIF_PLANES_YUV,
	AVIF_RESULT_OK,
	AVIF_RGB_FORMAT_RGBA,
	avifEncoder,
	avifEncoderCreate,
	avifEncoderDestroy,
	avifEncoderWrite,
	avifImage,
	avifImageAllocatePlanes,
	avifImageCreate,
	avifImageDestroy,
	avifImageRGBToYUV,
	avifRGBImage,
	avifRWData,
	avifRWDataFree,
};
use std::{
	convert::TryFrom,
	num::NonZeroU8,
};



/// # Avif Image.
///
/// This holds a YUV copy of the image, which is created in a roundabout way
/// by converting a raw slice into an RGB image. Haha.
///
/// The struct includes initialization helpers, but exists primarily for
/// garbage cleanup.
struct AvifImage(*mut avifImage);

impl TryFrom<&TreatedSource> for AvifImage {
	type Error = RefractError;
	fn try_from(src: &TreatedSource) -> Result<Self, Self::Error> {
		// Make sure dimensions fit u32.
		let (width, height) = src.dimensions();
		let width = u32::try_from(width).map_err(|_| RefractError::Encode)?;
		let height = u32::try_from(height).map_err(|_| RefractError::Encode)?;

		// Grab the buffer.
		let raw: &[u8] = src.buffer();

		// Make an "avifRGBImage" with that buffer.
		let rgb = avifRGBImage {
			width,
			height,
			depth: 8,
			format: AVIF_RGB_FORMAT_RGBA,
			chromaUpsampling: AVIF_CHROMA_UPSAMPLING_BILINEAR,
			ignoreAlpha: ! src.color().has_alpha() as _,
			alphaPremultiplied: 0,
			pixels: raw.as_ptr() as *mut u8,
			rowBytes: 4 * width,
		};

		// Make a YUV version of the same.
		let yuv = unsafe {
			let tmp = avifImageCreate(
				i32::try_from(width).map_err(|_| RefractError::Encode)?,
				i32::try_from(height).map_err(|_| RefractError::Encode)?,
				8, // Depth.
				1, // YUV444 = 1_i32
			);
			avifImageAllocatePlanes(tmp, AVIF_PLANES_YUV as _);
			avifImageRGBToYUV(tmp, &rgb);
			tmp
		};

		Ok(Self(yuv))
	}
}

impl Drop for AvifImage {
	#[inline]
	fn drop(&mut self) { unsafe { avifImageDestroy(self.0); } }
}



/// # AVIF Encoer.
///
/// This wraps the AVIF encoder. It primarily exists to give us a way to free
/// resources on drop, but also handles setup.
struct AvifEncoder(*mut avifEncoder);

impl AvifEncoder {
	#[allow(clippy::cast_possible_truncation)] // It fits.
	#[allow(clippy::cast_possible_wrap)]
	/// # New Instance.
	fn new(width: usize, height: usize, quality: NonZeroU8) -> Result<Self, RefractError> {
		// Convert quality to quantizers. AVIF is so convoluted...
		let (q, aq) = quality_to_quantizers(quality);

		// Total threads.
		let cpus = num_cpus::get();
		let threads = i32::try_from(cpus).map_err(|_| RefractError::Encode)?;

		// Start up the encoder!
		let encoder = unsafe { avifEncoderCreate() };
		unsafe {
			(*encoder).codecChoice = AVIF_CODEC_CHOICE_RAV1E;
			(*encoder).maxThreads = threads;

			(*encoder).minQuantizer = i32::from(q);
			(*encoder).maxQuantizer = i32::from(q);

			(*encoder).minQuantizerAlpha = i32::from(aq);
			(*encoder).maxQuantizerAlpha = i32::from(aq);

			// There is a speed 0, but it is brutally slow and has very little
			// benefit.
			(*encoder).speed = 1;

			// Enable tiling if we are multi-threaded. We want to try to keep
			// the combined X/Y value equal to the total number of threads,
			// while also ensuring we aren't trying to divide a small dimension
			// into too many chunks.
			if cpus > 1 {
				let tiles_x;
				let tiles_y;

				// Prioritize carving up width.
				if width >= height {
					// The magic "128" number is 2^6, where 6 is the absolute
					// maximum accepted tiling value.
					tiles_x = cpus.min(num_integer::div_floor(width, 128)).max(1);
					tiles_y = num_integer::div_floor(cpus, tiles_x)
						.min(num_integer::div_floor(height, 128))
						.max(1);
				}
				// Prioritize carving up the height.
				else {
					tiles_y = cpus.min(num_integer::div_floor(height, 128)).max(1);
					tiles_x = num_integer::div_floor(cpus, tiles_y)
						.min(num_integer::div_floor(width, 128))
						.max(1);
				}

				// We can only split up to 6 times, so cap values thusly.
				if tiles_x > 1 || tiles_y > 1 {
					(*encoder).tileRowsLog2 = 6.min(tiles_x) as i32;
					(*encoder).tileColsLog2 = 6.min(tiles_y) as i32;
				}
			}
		};

		Ok(Self(encoder))
	}
}

impl Drop for AvifEncoder {
	#[inline]
	fn drop(&mut self) { unsafe { avifEncoderDestroy(self.0); } }
}



/// # Data Struct.
///
/// This wrapper only exists to provide garbage cleanup.
struct AvifData(avifRWData);

impl Drop for AvifData {
	#[inline]
	fn drop(&mut self) { unsafe { avifRWDataFree(&mut self.0); } }
}



/// # Make Lossy.
///
/// Generate an `AVIF` image at a given quality size.
///
/// The quality passed should be the opposite of the quantizer scale used by
/// `libavif`, i.e. 0 is the worst and 63 is the best. (We'll flip it later
/// on.)
///
/// See [`quality_to_quantizers`] for more information.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossy(img: &TreatedSource, quality: NonZeroU8) -> Result<Vec<u8>, RefractError> {
	let image = AvifImage::try_from(img)?;
	let (width, height) = img.dimensions();
	let encoder = AvifEncoder::new(width, height, quality)?;
	let mut data = AvifData(avifRWData::default());

	// Encode!
	if AVIF_RESULT_OK != unsafe { avifEncoderWrite(encoder.0, image.0, &mut data.0) } {
		return Err(RefractError::Encode);
	}

	// Grab the output.
	let output: Vec<u8> = unsafe {
		std::slice::from_raw_parts(data.0.data, data.0.size).to_vec()
	};

	// This shouldn't be empty, but just in case...
	if output.is_empty() { Err(RefractError::Encode) }
	else { Ok(output) }
}

#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
/// # Quality to Quantizer(s).
///
/// This converts the quality stepping from [`OutputIter`] into appropriate
/// `libavif` quantizers.
///
/// The first step is to flip the provided value as [`OutputIter`] and
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

	// Alpha follows a neat little formula stolen from `ravif`. It is a lot
	// easier on the brain to recalibrate the value to be out of 100, then
	// re-recalibrate it to be out of 63.
	let aq = ratio_of(quality.get(), 63, 100);
	let aq = num_integer::div_floor(aq + 100, 2).min(
		aq + num_integer::div_floor(aq, 4) + 2
	);
	let aq = 63 - ratio_of(aq, 100, 63);

	(q, aq)
}

#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
#[inline]
/// # Ratio Of.
///
/// This simply takes a fraction, multiplies it against a new base, and returns
/// that value. It's a bit verbose, so is offloaded to its own place.
fn ratio_of(e: u8, d: u8, base: u8) -> u8 {
	(f32::from(e.min(d)) / f32::from(d) * f32::from(base))
		.max(0.0)
		.min(f32::from(base)) as u8
}
