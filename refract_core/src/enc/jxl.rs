/*!
# `Refract`: `JPEG XL` Handling
*/

use crate::{
	Image,
	Output,
	OutputKind,
	RefractError,
};
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
use std::{
	convert::TryFrom,
	ffi::c_void,
	num::NonZeroU8,
};



/// # Hold the Encoder.
///
/// This wrapper exists solely to help with drop cleanup.
struct LibJxlEncoder(*mut JxlEncoder);

impl LibJxlEncoder {
	/// # New instance!
	fn new() -> Result<Self, RefractError> {
		let enc = unsafe { JxlEncoderCreate(std::ptr::null()) };
		if enc.is_null() { Err(RefractError::Encode) }
		else { Ok(Self(enc)) }
	}

	/// # Set Basic Info.
	fn set_basic_info(&self, width: u32, height: u32, alpha: bool) -> Result<(), RefractError> {
		// Set up JPEG XL's "basic info" struct.
		let mut basic_info = unsafe { JxlBasicInfo::new_uninit().assume_init() };
		basic_info.xsize = width;
		basic_info.ysize = height;
		basic_info.uses_original_profile = false as _;
		basic_info.have_container = false as _;

		basic_info.bits_per_sample = 8;
		basic_info.exponent_bits_per_sample = 0;
		basic_info.alpha_premultiplied = false as _;
		basic_info.alpha_exponent_bits = 0;

		if alpha {
			basic_info.num_extra_channels = 1;
			basic_info.alpha_bits = 8;
		}
		else {
			basic_info.num_extra_channels = 0;
			basic_info.alpha_bits = 0;
		}

		maybe_die(&unsafe { JxlEncoderSetBasicInfo(self.0, &basic_info) })
	}

	/// # Write.
	fn write(&self) -> Result<Box<[u8]>, RefractError> {
		// Set up a write buf, starting with 1MB.
		const CHUNK_SIZE: usize = 1024 * 1024;
		let mut buf = vec![0; CHUNK_SIZE];
		let mut next_out = buf.as_mut_ptr().cast();
		let mut avail_out = CHUNK_SIZE;

		// Process the output.
		let mut status;
		loop {
			status = unsafe {
				JxlEncoderProcessOutput(self.0, &mut next_out, &mut avail_out)
			};
			if status != JxlEncoderStatus::NeedMoreOutput {
				break;
			}

			unsafe {
				let offset = next_out.offset_from(buf.as_ptr());
				buf.resize(buf.len() * 2, 0);
				next_out = (buf.as_mut_ptr()).offset(offset);
				avail_out = buf.len() - usize::try_from(offset).map_err(|_| RefractError::Encode)?;
			}
		}
		maybe_die(&status)?;

		// Adjust the buf accordingly.
		let len = usize::try_from(unsafe { next_out.offset_from(buf.as_ptr()) })
			.map_err(|_| RefractError::Encode)?;
		buf.truncate(len);
		Ok(buf.into_boxed_slice())
	}
}

impl Drop for LibJxlEncoder {
	#[inline]
	fn drop(&mut self) { unsafe { JxlEncoderDestroy(self.0) }; }
}



/// # Hold the Thread Runner.
///
/// This wrapper exists solely to help with drop cleanup.
struct LibJxlThreadParallelRunner(*mut c_void);

impl LibJxlThreadParallelRunner {
	/// # New instance!
	fn new() -> Result<Self, RefractError> {
		let threads = unsafe {
			JxlThreadParallelRunnerCreate(std::ptr::null(), num_cpus::get())
		};
		if threads.is_null() { Err(RefractError::Encode) }
		else { Ok(Self(threads)) }
	}
}

impl Drop for LibJxlThreadParallelRunner {
	#[inline]
	fn drop(&mut self) {
		unsafe { JxlThreadParallelRunnerDestroy(self.0) };
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
pub(super) fn make_lossy(img: &Image, quality: NonZeroU8) -> Result<Output, RefractError> {
	encode(img, Some(quality))
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
pub(super) fn make_lossless(img: &Image) -> Result<Output, RefractError> {
	encode(img, None)
}



/// # Encode.
///
/// This stitches all the pieces together. Who would have thought a
/// convoluted format like JPEG XL would require so many steps to produce?!
fn encode(img: &Image, quality: Option<NonZeroU8>) -> Result<Output, RefractError> {
	// Initialize the encoder.
	let enc = LibJxlEncoder::new()?;

	let color = img.color_kind();

	// Hook in parallelism.
	let runner = LibJxlThreadParallelRunner::new()?;
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
			color.is_greyscale()
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
	enc.set_basic_info(img.width_u32()?, img.height_u32()?, color.has_alpha())?;

	// Set up a "frame".
	let pixel_format = JxlPixelFormat {
		num_channels: color.color_channels() + color.extra_channels(),
		data_type: JxlDataType::Uint8,
		endianness: JxlEndianness::Native,
		align: 0,
	};

	let data: &[u8] = &*img;
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

	// Done!
	Output::new(
		enc.write()?,
		quality.unwrap_or_else(|| OutputKind::Jxl.lossless_quality())
	)
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
