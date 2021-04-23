/*!
# `Refract`: `JPEG XL` Handling
*/

use crate::RefractError;
use imgref::Img;
use jpegxl_sys::{
	JxlBasicInfo,
	JxlColorEncoding,
	JxlColorEncodingSetToSRGB,
	JxlDataType,
	JxlEncoder,
	JxlEncoderAddImageFrame,
	JxlEncoderCloseInput,
	JxlEncoderCreate,
	JxlEncoderDestroy,
	JxlEncoderOptions,
	JxlEncoderOptionsCreate,
	JxlEncoderOptionsSetDecodingSpeed,
	JxlEncoderOptionsSetDistance,
	JxlEncoderOptionsSetEffort,
	JxlEncoderOptionsSetLossless,
	JxlEncoderProcessOutput,
	JxlEncoderSetBasicInfo,
	JxlEncoderSetColorEncoding,
	JxlEncoderSetParallelRunner,
	JxlEncoderStatus,
	JxlEndianness,
	JxlPixelFormat,
	NewUninit,
	thread_runner::{
		JxlThreadParallelRunner,
		JxlThreadParallelRunnerCreate,
		JxlThreadParallelRunnerDestroy,
	},
};
use rgb::RGBA8;
use std::{
	convert::TryFrom,
	ffi::c_void,
	num::NonZeroU8,
};



#[derive(Debug, Clone, Copy)]
/// # Source Image Color.
///
/// We're always starting from `RGBA`, but we can help optimize instructions by
/// pre-calculating which channels are actually used.
enum JxlImageColor {
	Grey,
	GreyAlpha,
	Rgb,
	Rgba,
}

impl From<Img<&[RGBA8]>> for JxlImageColor {
	fn from(img: Img<&[RGBA8]>) -> Self {
		let alpha = img.pixels().any(|p| p.a != 255);
		let grey = img.pixels().all(|p| p.r == p.g && p.r == p.b);

		if alpha && grey { Self::GreyAlpha }
		else if alpha { Self::Rgba }
		else if grey { Self::Grey }
		else { Self::Rgb }
	}
}

impl JxlImageColor {
	/// # Color channels.
	const fn color_channels(self) -> u32 {
		match self {
			Self::Grey | Self::GreyAlpha => 1,
			Self::Rgb | Self::Rgba => 3,
		}
	}

	/// # Extra channels.
	const fn extra_channels(self) -> u32 {
		match self {
			Self::GreyAlpha | Self::Rgba => 1,
			_ => 0,
		}
	}

	/// # Is Greyscale?
	const fn is_greyscale(self) -> bool {
		matches!(self, Self::Grey | Self::GreyAlpha)
	}

	/// # Has Alpha?
	const fn has_alpha(self) -> bool {
		matches!(self, Self::GreyAlpha | Self::Rgba)
	}
}



#[derive(Debug)]
/// # Source Image.
///
/// This struct holds the basic source image as a contiguous byte slice along
/// with some reference metadata.
///
/// The bytes themselves will vary based on the color type; it could be
/// anywhere between `(width * height)` and `(width * height * 4)` in size. `R`
/// is always first, and `A`, if present, is always last.
struct JxlImage {
	data: Vec<u8>,
	width: u32,
	height: u32,
	color: JxlImageColor,
}

impl TryFrom<Img<&[RGBA8]>> for JxlImage {
	type Error = RefractError;
	fn try_from(img: Img<&[RGBA8]>) -> Result<Self, Self::Error> {
		// Parse out the dimensions.
		let width = u32::try_from(img.width()).map_err(|_| RefractError::Encode)?;
		let height = u32::try_from(img.height()).map_err(|_| RefractError::Encode)?;
		let color = JxlImageColor::from(img);

		// Convert the pixels to a contiguous buffer.
		let data: Vec<u8> = match color {
			JxlImageColor::Grey => img.pixels().map(|p| p.r).collect(),
			JxlImageColor::GreyAlpha => img.pixels().fold(
				Vec::with_capacity((width * height) as usize * 2),
				|mut acc, p| {
					acc.extend_from_slice(&[p.r, p.a]);
					acc
				}
			),
			JxlImageColor::Rgb => img.pixels().fold(
				Vec::with_capacity((width * height) as usize * 3),
				|mut acc, p| {
					acc.extend_from_slice(&[p.r, p.g, p.b]);
					acc
				}
			),
			JxlImageColor::Rgba => img.pixels().fold(
				Vec::with_capacity((width * height) as usize * 4),
				|mut acc, p| {
					acc.extend_from_slice(&[p.r, p.g, p.b, p.a]);
					acc
				}
			),
		};

		// This shouldn't happen, but just in case...
		if data.is_empty() {
			Err(RefractError::Encode)
		}
		else {
			Ok(Self {
				data,
				width,
				height,
				color,
			})
		}
	}
}

impl JxlImage {
	/// # Encode.
	fn encode(&self, quality: Option<NonZeroU8>) -> Result<Vec<u8>, RefractError> {
		// Initialize the encoder.
		let enc = JxlImageEncoder::new()?;

		// Hook in parallelism.
		let runner = JxlImageEncoderThreads::new()?;
		maybe_die(unsafe {
			&JxlEncoderSetParallelRunner(
				enc.0,
				Some(JxlThreadParallelRunner),
				runner.0
			)
		})?;

		// Initialize the options wrapper.
		let options: *mut JxlEncoderOptions = unsafe {
			JxlEncoderOptionsCreate(enc.0, std::ptr::null())
		};

		// Color handling.
		let color_encoding = unsafe {
			let mut color_encoding = JxlColorEncoding::new_uninit();
			JxlColorEncodingSetToSRGB(
				color_encoding.as_mut_ptr(),
				self.color.is_greyscale()
			);
			color_encoding.assume_init()
		};

		maybe_die(&unsafe { JxlEncoderSetColorEncoding(enc.0, &color_encoding) })?;

		// Quality. We have to convert the NonZeroU8 to a float because JPEG XL
		// weird. After translation, 0.0 is lossless, 15.0 is garbage.
		match quality.map(NonZeroU8::get) {
			// Lossy distance.
			Some(q) if q < 150 => maybe_die(&unsafe {
				JxlEncoderOptionsSetDistance(options, f32::from(150_u8 - q) / 10.0)
			})?,
			// Lossless.
			_ => maybe_die(&unsafe { JxlEncoderOptionsSetLossless(options, true) })?,
		};

		// Effort. 9 == Tortoise.
		maybe_die(&unsafe { JxlEncoderOptionsSetEffort(options, 9) })?;

		// Decoding speed. 0 == Highest quality.
		maybe_die(&unsafe { JxlEncoderOptionsSetDecodingSpeed(options, 0) })?;

		// Set up JPEG XL's "basic info" struct.
		let mut basic_info = unsafe { JxlBasicInfo::new_uninit().assume_init() };
		basic_info.xsize = self.width;
		basic_info.ysize = self.height;
		basic_info.uses_original_profile = false as _;
		basic_info.have_container = false as _;

		basic_info.bits_per_sample = 8;
		basic_info.exponent_bits_per_sample = 0;
		basic_info.alpha_premultiplied = false as _;
		basic_info.alpha_exponent_bits = 0;

		if self.color.has_alpha() {
			basic_info.num_extra_channels = 1;
			basic_info.alpha_bits = 8;
		}
		else {
			basic_info.num_extra_channels = 0;
			basic_info.alpha_bits = 0;
		}

		maybe_die(&unsafe { JxlEncoderSetBasicInfo(enc.0, &basic_info) })?;

		// Set up a "frame".
		let pixel_format = JxlPixelFormat {
			num_channels: self.color.color_channels() + self.color.extra_channels(),
			data_type: JxlDataType::Uint8,
			endianness: JxlEndianness::Native,
			align: 0,
		};

		let data: &[u8] = &self.data;
		maybe_die(&unsafe {
			JxlEncoderAddImageFrame(
				options,
				&pixel_format,
				data.as_ptr().cast(),
				std::mem::size_of_val(data),
			)
		})?;

		// Finalize the encoder.
		unsafe { JxlEncoderCloseInput(enc.0) };

		// Set up a write buffer, starting with 1MB.
		let chunk_size = 1024 * 1024;
		let mut buffer = vec![0; chunk_size];
		let mut next_out = buffer.as_mut_ptr().cast();
		let mut avail_out = chunk_size;

		// Process the output.
		let mut status;
		loop {
			status = unsafe {
				JxlEncoderProcessOutput(enc.0, &mut next_out, &mut avail_out)
			};
			if status != JxlEncoderStatus::NeedMoreOutput {
				break;
			}

			unsafe {
				let offset = next_out.offset_from(buffer.as_ptr());
				buffer.resize(buffer.len() * 2, 0);
				next_out = (buffer.as_mut_ptr()).offset(offset);
				avail_out = buffer.len() - usize::try_from(offset).map_err(|_| RefractError::Encode)?;
			}
		}
		maybe_die(&status)?;

		// Adjust the buffer accordingly.
		let len: usize = usize::try_from(unsafe { next_out.offset_from(buffer.as_ptr()) })
			.map_err(|_| RefractError::Encode)?;
		buffer.truncate(len);

		// Done!
		if buffer.is_empty() { Err(RefractError::Encode) }
		else { Ok(buffer) }
	}
}



#[derive(Debug)]
/// # Hold the Encoder.
///
/// This wrapper exists solely to help with drop cleanup.
struct JxlImageEncoder(*mut JxlEncoder);

impl JxlImageEncoder {
	/// # New instance!
	fn new() -> Result<Self, RefractError> {
		let enc = unsafe { JxlEncoderCreate(std::ptr::null()) };
		if enc.is_null() { Err(RefractError::Encode) }
		else { Ok(Self(enc)) }
	}
}

impl Drop for JxlImageEncoder {
	#[inline]
	fn drop(&mut self) {
		unsafe { JxlEncoderDestroy(self.0) };
	}
}



#[derive(Debug)]
/// # Hold the Thread Runner.
///
/// This wrapper exists solely to help with drop cleanup.
struct JxlImageEncoderThreads(*mut c_void);

impl JxlImageEncoderThreads {
	/// # New instance!
	fn new() -> Result<Self, RefractError> {
		let threads = unsafe {
			JxlThreadParallelRunnerCreate(std::ptr::null(), num_cpus::get())
		};
		if threads.is_null() { Err(RefractError::Encode) }
		else { Ok(Self(threads)) }
	}
}

impl Drop for JxlImageEncoderThreads {
	#[inline]
	fn drop(&mut self) {
		unsafe { JxlThreadParallelRunnerDestroy(self.0) };
	}
}



/// # Verify Encoder Status.
///
/// Most `JPEG XL` API methods return a status; this converts unsuccessful
/// statuses to a proper Rust error.
const fn maybe_die(res: &JxlEncoderStatus) -> Result<(), RefractError> {
	match res {
		JxlEncoderStatus::Success => Ok(()),
		_ => Err(RefractError::Encode),
	}
}

#[inline]
/// # Make Lossy.
///
/// Generate a lossy `JPEG XL` image at a given quality size.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossy(img: Img<&[RGBA8]>, quality: NonZeroU8) -> Result<Vec<u8>, RefractError> {
	let enc = JxlImage::try_from(img)?;
	enc.encode(Some(quality))
}

#[inline]
/// # Make Lossy.
///
/// Generate a lossless `JPEG XL`.
///
/// ## Errors
///
/// This returns an error in cases where the resulting file size is larger
/// than the source or previous best, or if there are any problems
/// encountered during encoding or saving.
pub(super) fn make_lossless(img: Img<&[RGBA8]>) -> Result<Vec<u8>, RefractError> {
	let enc = JxlImage::try_from(img)?;
	enc.encode(None)
}
