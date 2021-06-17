/*!
# `Refract`: `AVIF` Handling
*/

use crate::{
	FLAG_AVIF_RGB,
	FLAG_AVIF_ROUND_3,
	Input,
	Output,
	RefractError,
	traits::Encoder,
};
use libavif_sys::{
	AVIF_CHROMA_SAMPLE_POSITION_COLOCATED,
	AVIF_CHROMA_UPSAMPLING_BILINEAR,
	AVIF_CODEC_CHOICE_RAV1E,
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
	avifEncoder,
	avifEncoderCreate,
	avifEncoderDestroy,
	avifEncoderWrite,
	avifImage,
	avifImageCreate,
	avifImageDestroy,
	avifImageRGBToYUV,
	avifResult,
	avifRGBImage,
	avifRWData,
	avifRWDataFree,
};
use std::{
	convert::TryFrom,
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

#[cfg(feature = "decode_ng")]
use libavif_sys::{
	AVIF_CODEC_CHOICE_DAV1D,
	avifDecoder,
	avifDecoderCreate,
	avifDecoderDestroy,
	avifDecoderReadMemory,
	avifImageCreateEmpty,
	avifImageYUVToRGB,
	avifRGBImageAllocatePixels,
	avifRGBImageFreePixels,
	avifRGBImageSetDefaults,
};


// These constants are used by the tiling functions.
const MAX_TILE_AREA: usize = 4096 * 2304;
const MAX_TILE_COLS: usize = 64;
const MAX_TILE_ROWS: usize = 64;
const MAX_TILE_WIDTH: usize = 4096;
const SB_SIZE_LOG2: usize = 6;



#[allow(unreachable_pub)] // Unsolvable?
/// # AVIF Image.
pub struct ImageAvif;

#[cfg(feature = "decode_ng")]
impl Decoder for ImageAvif {
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
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
			avifRGBImageAllocatePixels(&mut rgb.0);
			if AVIF_RESULT_OK != avifImageYUVToRGB(image.0, &mut rgb.0) {
				return Err(RefractError::Decode);
			}

			// Done!
			rgb
		};

		// Make sure the dimensions fit `usize`.
		let width = usize::try_from(rgb.0.width)
			.map_err(|_| RefractError::Overflow)?;

		let height = usize::try_from(rgb.0.height)
			.map_err(|_| RefractError::Overflow)?;

		let size = width.checked_mul(height)
			.and_then(|x| x.checked_mul(4))
			.ok_or(RefractError::Overflow)?;

		// Steal the buffer.
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
	const MAX_QUALITY: NonZeroU8 = unsafe { NonZeroU8::new_unchecked(63) };

	/// # Encode Lossy.
	fn encode_lossy(img: &Input, candidate: &mut Output, quality: NonZeroU8, flags: u8)
	-> Result<(), RefractError> {
		let image = LibAvifImage::new(img, flags)?;
		let encoder = LibAvifEncoder::try_from(quality)?;

		// Configure tiling.
		if 0 == FLAG_AVIF_ROUND_3 & flags {
			if let Some((x, y)) = tiles(img.width(), img.height()) {
				unsafe {
					(*encoder.0).tileRowsLog2 = x;
					(*encoder.0).tileColsLog2 = y;
				}
			}
		}

		// Encode!
		let mut data = LibAvifRwData(avifRWData::default());
		maybe_die(unsafe { avifEncoderWrite(encoder.0, image.0, &mut data.0) })?;

		// Grab the output.
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



#[cfg(feature = "decode_ng")]
/// # AVIF Decoder.
///
/// This wraps the AVIF decoder. It exists solely for garbage cleanup.
struct LibAvifDecoder(*mut avifDecoder);

#[cfg(feature = "decode_ng")]
impl LibAvifDecoder {
	/// # New.
	fn new() -> Result<Self, RefractError> {
		let decoder = unsafe { avifDecoderCreate() };
		if decoder.is_null() {
			return Err(RefractError::Decode);
		}

		// Set up the threads.
		let threads = i32::try_from(num_cpus::get())
			.unwrap_or(1)
			.max(1);

		unsafe {
			(*decoder).codecChoice = AVIF_CODEC_CHOICE_DAV1D;
			(*decoder).maxThreads = threads;
		}

		Ok(Self(decoder))
	}
}

#[cfg(feature = "decode_ng")]
impl Drop for LibAvifDecoder {
	#[inline]
	fn drop(&mut self) { unsafe { avifDecoderDestroy(self.0); } }
}



/// # AVIF Encoder.
///
/// This wraps the AVIF encoder. It primarily exists to give us a way to free
/// resources on drop, but also handles setup.
struct LibAvifEncoder(*mut avifEncoder);

impl TryFrom<NonZeroU8> for LibAvifEncoder {
	type Error = RefractError;

	/// # New Instance.
	fn try_from(quality: NonZeroU8) -> Result<Self, RefractError> {
		// Convert quality to quantizers. AVIF is so convoluted...
		let (q, aq) = quality_to_quantizers(quality);

		// Total threads.
		let threads = i32::try_from(num_cpus::get())
			.unwrap_or(1)
			.max(1);

		// Start up the encoder!
		let encoder = unsafe { avifEncoderCreate() };
		if encoder.is_null() { return Err(RefractError::Encode); }

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
		};

		Ok(Self(encoder))
	}
}

impl Drop for LibAvifEncoder {
	#[inline]
	fn drop(&mut self) { unsafe { avifEncoderDestroy(self.0); } }
}



/// # Avif Image.
///
/// The struct includes initialization helpers, but exists primarily for
/// garbage cleanup.
struct LibAvifImage(*mut avifImage);

impl LibAvifImage {
	#[allow(clippy::cast_possible_truncation)] // The values are purpose-made.
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
		let raw: &[u8] = &*src;
		let rgb = avifRGBImage {
			width,
			height,
			depth: 8,
			format: AVIF_RGB_FORMAT_RGBA,
			chromaUpsampling: AVIF_CHROMA_UPSAMPLING_BILINEAR,
			ignoreAlpha: ! src.has_alpha() as _,
			alphaPremultiplied: 0,
			pixels: raw.as_ptr() as *mut u8,
			rowBytes: 4 * width,
		};

		// And convert it to YUV.
		let yuv = unsafe {
			let tmp = avifImageCreate(
				src.width_i32()?,
				src.height_i32()?,
				8, // Depth.
				if greyscale { AVIF_PIXEL_FORMAT_YUV400 }
				else { AVIF_PIXEL_FORMAT_YUV444 }
			);

			// This shouldn't happen, but could, maybe.
			if tmp.is_null() { return Err(RefractError::Encode); }

			(*tmp).yuvRange =
				if limited { AVIF_RANGE_LIMITED }
				else { AVIF_RANGE_FULL };
			(*tmp).alphaRange = AVIF_RANGE_FULL;

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

	#[cfg(feature = "decode_ng")]
	/// # Empty.
	fn empty() -> Result<Self, RefractError> {
		let image = unsafe { avifImageCreateEmpty() };
		if image.is_null() { Err(RefractError::Decode) }
		else { Ok(Self(image)) }
	}
}

impl Drop for LibAvifImage {
	#[inline]
	fn drop(&mut self) { unsafe { avifImageDestroy(self.0); } }
}



#[cfg(feature = "decode_ng")]
/// # Avif RGB Image.
///
/// This struct exists only for garbage collection purposes. It is used for
/// decoding.
struct LibAvifRGBImage(avifRGBImage);

#[cfg(feature = "decode_ng")]
impl Default for LibAvifRGBImage {
	#[inline]
	fn default() -> Self { Self(avifRGBImage::default()) }
}

#[cfg(feature = "decode_ng")]
impl Drop for LibAvifRGBImage {
	fn drop(&mut self) { unsafe { avifRGBImageFreePixels(&mut self.0); } }
}



/// # Data Struct.
///
/// This wrapper only exists to provide garbage cleanup.
struct LibAvifRwData(avifRWData);

impl Drop for LibAvifRwData {
	#[inline]
	fn drop(&mut self) { unsafe { avifRWDataFree(&mut self.0); } }
}



/// # Tiling Helper.
///
/// This struct exists solely to collect and hold basic image tiling variables
/// for use by [`tiles`].
///
/// It is a stripped down version of `rav1e`'s `TilingInfo` struct, containing
/// only the bits needed for x/y tile figuring.
struct LibAvifTiles {
	cols: usize,
	rows: usize,

	max_tile_cols_log2: usize,
	max_tile_rows_log2: usize,

	tile_rows_log2: usize,

	tile_width_sb: usize,
	tile_height_sb: usize,
}

impl LibAvifTiles {
	/// # Tile Info.
	fn from_target_tiles(
		frame_width: usize,
		frame_height: usize,
		tile_cols_log2: usize,
		tile_rows_log2: usize,
	) -> Option<Self> {
		// Align frames to the next multiple of 8.
		let frame_width = ceil_log2(frame_width, 3);
		let frame_height = ceil_log2(frame_height, 3);
		let frame_width_sb = align_shift_pow2(frame_width, SB_SIZE_LOG2);
		let frame_height_sb = align_shift_pow2(frame_height, SB_SIZE_LOG2);
		let sb_cols = align_shift_pow2(frame_width, SB_SIZE_LOG2);
		let sb_rows = align_shift_pow2(frame_height, SB_SIZE_LOG2);

		// Set up some hard-coded limits. These are mostly format-dictated.
		let max_tile_width_sb = MAX_TILE_WIDTH >> SB_SIZE_LOG2;
		let max_tile_area_sb = MAX_TILE_AREA >> (2 * SB_SIZE_LOG2);
		let min_tile_cols_log2 = tile_log2(max_tile_width_sb, sb_cols)?;
		let max_tile_cols_log2 = tile_log2(1, sb_cols.min(MAX_TILE_COLS))?;
		let max_tile_rows_log2 = tile_log2(1, sb_rows.min(MAX_TILE_ROWS))?;
		let min_tiles_log2 = min_tile_cols_log2
		  .max(tile_log2(max_tile_area_sb, sb_cols * sb_rows)?);

		let mut tile_cols_log2 =
			tile_cols_log2.max(min_tile_cols_log2).min(max_tile_cols_log2);
		let tile_width_sb_pre = align_shift_pow2(sb_cols, tile_cols_log2);

		let tile_width_sb = tile_width_sb_pre;

		let cols = dactyl::div_usize(
			frame_width_sb + tile_width_sb - 1,
			tile_width_sb
		);

		// Adjust tile_cols_log2 to account for rounding.
		tile_cols_log2 = tile_log2(1, cols)?;
		if tile_cols_log2 < min_tile_cols_log2 {
			return None;
		}

		let min_tile_rows_log2 =
			if min_tiles_log2 > tile_cols_log2 {
				min_tiles_log2 - tile_cols_log2
			}
			else { 0 };

		let tile_rows_log2 = tile_rows_log2
			.max(min_tile_rows_log2)
			.min(max_tile_rows_log2);
		let tile_height_sb = align_shift_pow2(sb_rows, tile_rows_log2);

		let rows = dactyl::div_usize(
			frame_height_sb + tile_height_sb - 1,
			tile_height_sb
		);

		// We're done!
		Some(Self {
			cols,
			rows,
			max_tile_cols_log2,
			max_tile_rows_log2,
			tile_rows_log2,
			tile_width_sb,
			tile_height_sb,
		})
	}
}



#[inline]
/// # Align and Shift to Power of 2.
const fn align_shift_pow2(a: usize, n: usize) -> usize { (a + (1 << n) - 1) >> n }

#[inline]
/// # Ceiled Log2.
const fn ceil_log2(a: usize, n: usize) -> usize { floor_log2(a + (1 << n) - 1, n) }

#[inline]
/// # Floored Log2.
const fn floor_log2(a: usize, n: usize) -> usize { a & !((1 << n) - 1) }

#[inline]
/// # Verify Encoder Status.
///
/// This converts unsuccessful AVIF system function results into proper Rust
/// errors.
const fn maybe_die(res: avifResult) -> Result<(), RefractError> {
	if AVIF_RESULT_OK == res { Ok(()) }
	else { Err(RefractError::Encode) }
}

#[allow(clippy::cast_sign_loss)]
#[allow(clippy::cast_possible_truncation)]
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
	let aq = dactyl::div_u8(aq + 100, 2).min(
		aq + dactyl::div_u8(aq, 4) + 2
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

/// Return the smallest value for `k` such that `blkSize << k` is greater
/// than or equal to `target`.
fn tile_log2(blk_size: usize, target: usize) -> Option<usize> {
	let mut k = 0;
	while (blk_size.checked_shl(k)?) < target {
		k += 1;
	}
	Some(k as usize)
}

#[must_use]
/// # Tile Rows and Columns.
///
/// This is essentially a stripped-down version of the formula `rav1e` uses
/// to convert a singular "tiles" setting into separate row and column tile
/// settings.
///
/// It is a lot of figuring just to split a number, but greatly improves
/// encoding performance.
///
/// There is some compression savings tradeoff to be considered. Refract
/// actually re-runs the "best" image again at the end to see if it can
/// recover the losses (while still gaining the speed benefits from all
/// the other runs).
///
/// Not all images are suitable for tiling; this will return `None` in such
/// cases. If it returns `Some`, at least one value will be > 0.
fn tiles(width: usize, height: usize) -> Option<(i32, i32)> {
	// The tiling values should roughly match the number of CPUs, while
	// also not exceeding 6 (2^6 = 128). Aside from 6 being a hardcoded
	// limit, it isn't worth generating a million tiny tiles if the CPU has
	// to wait to deal with them.
	let tiles_max: usize = num_cpus::get()
		.min(dactyl::div_usize(width * height, 128 * 128));
	if tiles_max < 2 { return None; }

	// A starting point.
	let mut tile_rows_log2 = 0;
	let mut tile_cols_log2 = 0;
	let mut tiling = LibAvifTiles::from_target_tiles(
		width,
		height,
		tile_cols_log2,
		tile_rows_log2,
	)?;

	// Loop until the limits are reached.
	while
		(tile_rows_log2 < tiling.max_tile_rows_log2) ||
		(tile_cols_log2 < tiling.max_tile_cols_log2)
	{
		tiling = LibAvifTiles::from_target_tiles(
			width,
			height,
			tile_cols_log2,
			tile_rows_log2,
		)?;

		// The end.
		if tiling.rows * tiling.cols >= tiles_max { break; }

		// Bump the row count.
		if
			tile_cols_log2 >= tiling.max_tile_cols_log2 ||
			(
				(tiling.tile_height_sb >= tiling.tile_width_sb) &&
				(tiling.tile_rows_log2 < tiling.max_tile_rows_log2)
			)
		{
			tile_rows_log2 += 1;
		}
		// Bump the column count.
		else {
			tile_cols_log2 += 1;
		}
	}

	// Return what we've found if at least one of the values is non-zero
	// and both values fit within i32. (They shouldn't ever not fit, but
	// verbosity feels right after so much crunching.)
	if 0 < tile_rows_log2 || 0 < tile_cols_log2 {
		let rows = i32::try_from(tile_rows_log2).ok()?;
		let cols = i32::try_from(tile_cols_log2).ok()?;
		Some((rows, cols))
	}
	else { None }
}
