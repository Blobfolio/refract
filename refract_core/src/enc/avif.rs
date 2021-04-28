/*!
# `Refract`: `AVIF` Handling
*/

use crate::{
	Image,
	Output,
	RefractError,
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



// These constants are used by the tiling functions.
const MAX_TILE_AREA: usize = 4096 * 2304;
const MAX_TILE_COLS: usize = 64;
const MAX_TILE_ROWS: usize = 64;
const MAX_TILE_WIDTH: usize = 4096;
const SB_SIZE_LOG2: usize = 6;



/// # Avif Image.
///
/// This holds a YUV copy of the image, which is created in a roundabout way
/// by converting a raw slice into an RGB image. Haha.
///
/// The struct includes initialization helpers, but exists primarily for
/// garbage cleanup.
struct AvifImage(*mut avifImage);

impl TryFrom<&Image<'_>> for AvifImage {
	type Error = RefractError;
	fn try_from(src: &Image) -> Result<Self, Self::Error> {
		// Make sure dimensions fit u32.
		let width = u32::try_from(src.width()).map_err(|_| RefractError::Overflow)?;
		let height = u32::try_from(src.height()).map_err(|_| RefractError::Overflow)?;

		// Grab the buffer.
		let raw: &[u8] = &*src;

		// Make an "avifRGBImage" with that buffer.
		let rgb = avifRGBImage {
			width,
			height,
			depth: 8,
			format: AVIF_RGB_FORMAT_RGBA,
			chromaUpsampling: AVIF_CHROMA_UPSAMPLING_BILINEAR,
			ignoreAlpha: ! src.color_kind().has_alpha() as _,
			alphaPremultiplied: 0,
			pixels: raw.as_ptr() as *mut u8,
			rowBytes: 4 * width,
		};

		// Make a YUV version of the same.
		let yuv = unsafe {
			let tmp = avifImageCreate(
				i32::try_from(width).map_err(|_| RefractError::Overflow)?,
				i32::try_from(height).map_err(|_| RefractError::Overflow)?,
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

impl TryFrom<NonZeroU8> for AvifEncoder {
	type Error = RefractError;

	/// # New Instance.
	fn try_from(quality: NonZeroU8) -> Result<Self, RefractError> {
		// Convert quality to quantizers. AVIF is so convoluted...
		let (q, aq) = quality_to_quantizers(quality);

		// Total threads.
		let threads = i32::try_from(num_cpus::get()).map_err(|_| RefractError::Encode)?;

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


/// # Tiling Helper.
///
/// This struct exists solely to collect and hold basic image tiling variables
/// for use by [`tiles`].
///
/// It is a stripped down version of `rav1e`'s `TilingInfo` struct, containing
/// only the bits needed for x/y tile figuring.
struct TilingLite {
	cols: usize,
	rows: usize,

	max_tile_cols_log2: usize,
	max_tile_rows_log2: usize,

	tile_rows_log2: usize,

	tile_width_sb: usize,
	tile_height_sb: usize,
}

impl TilingLite {
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

		let cols = num_integer::div_floor(
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

		let rows = num_integer::div_floor(
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
pub(super) fn make_lossy(
	img: &Image,
	quality: NonZeroU8,
	tiling: bool
) -> Result<Output, RefractError> {
	let image = AvifImage::try_from(img)?;
	let encoder = AvifEncoder::try_from(quality)?;

	// Configure tiling.
	if tiling {
		if let Some((x, y)) = tiles(img.width(), img.height()) {
			unsafe {
				(*encoder.0).tileRowsLog2 = x;
				(*encoder.0).tileColsLog2 = y;
			}
		}
	}

	let mut data = AvifData(avifRWData::default());

	// Encode!
	if AVIF_RESULT_OK != unsafe { avifEncoderWrite(encoder.0, image.0, &mut data.0) } {
		return Err(RefractError::Encode);
	}

	// Grab the output.
	let raw: Box<[u8]> = unsafe {
		std::slice::from_raw_parts(data.0.data, data.0.size)
			.to_vec()
			.into_boxed_slice()
	};

	Output::new(raw, quality)
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
		.min(num_integer::div_floor(width * height, 128 * 128));
	if tiles_max < 2 { return None; }

	// A starting point.
	let mut tile_rows_log2 = 0;
	let mut tile_cols_log2 = 0;
	let mut tiling = TilingLite::from_target_tiles(
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
		tiling = TilingLite::from_target_tiles(
			width,
			height,
			tile_cols_log2,
			tile_rows_log2,
		)?;

		// The end.
		if tiling.rows * tiling.cols >= tiles_max { break; }

		// Bump the row count.
		if
			(
				(tiling.tile_height_sb >= tiling.tile_width_sb) &&
				(tiling.tile_rows_log2 < tiling.max_tile_rows_log2)
			) ||
			(tile_cols_log2 >= tiling.max_tile_cols_log2)
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
