/*!
# `Refract`: `JPEG XL` Handling
*/

use crate::{
	ColorKind,
	Input,
	NZ_150,
	Output,
	RefractError,
	traits::{
		Decoder,
		DecoderResult,
		Encoder,
	},
};
use jpegxl_sys::{
	color::color_encoding::JxlColorEncoding,
	common::types::{
		JxlBool,
		JxlEndianness,
		JxlDataType,
		JxlPixelFormat,
	},
	decode::{
		JxlColorProfileTarget,
		JxlDecoder,
		JxlDecoderCreate,
		JxlDecoderDestroy,
		JxlDecoderGetBasicInfo,
		JxlDecoderGetColorAsICCProfile,
		JxlDecoderGetICCProfileSize,
		JxlDecoderImageOutBufferSize,
		JxlDecoderProcessInput,
		JxlDecoderReset,
		JxlDecoderSetImageOutBuffer,
		JxlDecoderSetInput,
		JxlDecoderSetKeepOrientation,
		JxlDecoderStatus,
		JxlDecoderSubscribeEvents,
	},
	encoder::encode::{
		JxlColorEncodingSetToSRGB,
		JxlEncoder,
		JxlEncoderAddImageFrame,
		JxlEncoderCloseInput,
		JxlEncoderCreate,
		JxlEncoderDestroy,
		JxlEncoderFrameSettingId,
		JxlEncoderFrameSettings,
		JxlEncoderFrameSettingsCreate,
		JxlEncoderFrameSettingsSetOption,
		JxlEncoderInitBasicInfo,
		JxlEncoderProcessOutput,
		JxlEncoderSetBasicInfo,
		JxlEncoderSetColorEncoding,
		JxlEncoderSetExtraChannelDistance,
		JxlEncoderSetFrameDistance,
		JxlEncoderSetFrameLossless,
		JxlEncoderSetParallelRunner,
		JxlEncoderStatus,
		JxlEncoderUseContainer,
	},
	metadata::codestream_header::JxlBasicInfo,
	threads::thread_parallel_runner::{
		JxlThreadParallelRunner,
		JxlThreadParallelRunnerCreate,
		JxlThreadParallelRunnerDestroy,
	},
};
use std::{
	ffi::c_void,
	mem::MaybeUninit,
	num::{
		NonZeroU8,
		NonZeroUsize,
	},
};



/// # JPEG XL Image.
pub(crate) struct ImageJxl;

impl Decoder for ImageJxl {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	fn decode(raw: &[u8]) -> Result<DecoderResult, RefractError> {
		let decoder = LibJxlDecoder::new()?;
		let mut basic_info: Option<JxlBasicInfo> = None;
		let mut pixel_format: Option<JxlPixelFormat> = None;
		let mut icc_profile: Vec<u8> = Vec::new();

		// Get the buffer going.
		let mut buffer: Vec<u8> = Vec::new();
		let next_in = raw.as_ptr();
		let avail_in: usize = size_of_val(raw);
		// Safety: this is an FFI call…
		maybe_die_dec(unsafe { JxlDecoderSetInput(decoder.0, next_in, avail_in) })?;

		loop {
			// Safety: this is an FFI call…
			match unsafe { JxlDecoderProcessInput(decoder.0) } {
				JxlDecoderStatus::BasicInfo => {
					decoder.get_basic_info(
						&mut basic_info,
						&mut pixel_format
					)?;
				},
				JxlDecoderStatus::ColorEncoding => {
					decoder.get_icc_profile(
						&mut icc_profile
					)?;
				},
				JxlDecoderStatus::NeedImageOutBuffer => {
					decoder.output(
						pixel_format.as_ref().ok_or(RefractError::Decode)?,
						&mut buffer
					)?;
				},
				JxlDecoderStatus::FullImage => {},
				JxlDecoderStatus::Success => {
					// Safety: this is an FFI call…
					unsafe { JxlDecoderReset(decoder.0); }

					let info = basic_info.ok_or(RefractError::Decode)?;
					let width = usize::try_from(info.xsize)
						.map_err(|_| RefractError::Overflow)?;
					let height = usize::try_from(info.ysize)
						.map_err(|_| RefractError::Overflow)?;
					let size = width.checked_mul(height)
						.and_then(|x| x.checked_mul(4))
						.ok_or(RefractError::Overflow)?;

					if buffer.len() == size {
						let color = ColorKind::from_rgba(&buffer);
						return Ok((buffer, width, height, color));
					}

					return Err(RefractError::Decode);
				},
				_ => return Err(RefractError::Decode),
			}
		}
	}
}

impl Encoder for ImageJxl {
	/// # Maximum Quality.
	const MAX_QUALITY: NonZeroU8 = NZ_150;

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



/// # Hold the Decoder.
///
/// This wrapper exists solely to help with drop cleanup.
struct LibJxlDecoder(*mut JxlDecoder);

impl LibJxlDecoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # New Decoder.
	fn new() -> Result<Self, RefractError> {
		// Safety: this is an FFI call…
		let dec = unsafe { JxlDecoderCreate(std::ptr::null()) };
		if dec.is_null() {
			return Err(RefractError::Decode);
		}

		maybe_die_dec(
			// Safety: this is an FFI call…
			unsafe {
				JxlDecoderSubscribeEvents(
					dec,
					JxlDecoderStatus::BasicInfo as i32 |
					JxlDecoderStatus::ColorEncoding as i32 |
					JxlDecoderStatus::FullImage as i32
				)
			}
		)?;

		// Safety: this is an FFI call…
		maybe_die_dec(unsafe { JxlDecoderSetKeepOrientation(dec, JxlBool::True) })?;

		Ok(Self(dec))
	}

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Load Basic Info.
	fn get_basic_info(
		&self,
		basic_info: &mut Option<JxlBasicInfo>,
		pixel_format: &mut Option<JxlPixelFormat>,
	) -> Result<(), RefractError> {
		// Safety: these are FFI calls…
		*basic_info = Some(unsafe {
			let mut info = MaybeUninit::uninit();
			maybe_die_dec(JxlDecoderGetBasicInfo(self.0, info.as_mut_ptr()))?;
			info.assume_init()
		});

		*pixel_format = Some(JxlPixelFormat {
			num_channels: 4,
			data_type: JxlDataType::Uint8,
			endianness: JxlEndianness::Native,
			align: 0,
		});

		Ok(())
	}

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Load ICC Profile.
	fn get_icc_profile(&self, icc_profile: &mut Vec<u8>)
	-> Result<(), RefractError> {
		let mut icc_size = 0;

		maybe_die_dec(
			// Safety: this is an FFI call…
			unsafe {
				JxlDecoderGetICCProfileSize(
					self.0,
					JxlColorProfileTarget::Data,
					&mut icc_size,
				)
			}
		)?;

		icc_profile.resize(icc_size, 0);

		maybe_die_dec(
			// Safety: this is an FFI call…
			unsafe {
				JxlDecoderGetColorAsICCProfile(
					self.0,
					JxlColorProfileTarget::Data,
					icc_profile.as_mut_ptr(),
					icc_size,
				)
			}
		)?;

		Ok(())
	}

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Output.
	fn output(
		&self,
		pixel_format: &JxlPixelFormat,
		buffer: &mut Vec<u8>,
	) -> Result<(), RefractError> {
		let mut size = 0;
		// Safety: this is an FFI call…
		maybe_die_dec(unsafe {
			JxlDecoderImageOutBufferSize(self.0, pixel_format, &mut size)
		})?;

		buffer.resize(size, 0);
		maybe_die_dec(
			// Safety: this is an FFI call…
			unsafe {
				JxlDecoderSetImageOutBuffer(
					self.0,
					pixel_format,
					buffer.as_mut_ptr().cast(),
					size,
				)
			}
		)?;

		Ok(())
	}
}

impl Drop for LibJxlDecoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libjxl handles deallocation.
		unsafe { JxlDecoderDestroy(self.0); }
	}
}



/// # Hold the Encoder.
///
/// This wrapper exists solely to help with drop cleanup.
struct LibJxlEncoder(*mut JxlEncoder);

impl LibJxlEncoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # New instance!
	fn new() -> Result<Self, RefractError> {
		// Safety: this is an FFI call…
		let enc = unsafe { JxlEncoderCreate(std::ptr::null()) };
		if enc.is_null() { Err(RefractError::Encode) }
		else { Ok(Self(enc)) }
	}

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Set Basic Info.
	fn set_basic_info(&self, width: u32, height: u32, alpha: bool, grey: bool) -> Result<(), RefractError> {
		// Set up JPEG XL's "basic info" struct.
		// Safety: this is an FFI call…
		let mut basic_info = unsafe {
			let mut info = MaybeUninit::uninit();
			JxlEncoderInitBasicInfo(info.as_mut_ptr());
			info.assume_init()
		};

		basic_info.xsize = width;
		basic_info.ysize = height;
		basic_info.uses_original_profile = JxlBool::True;
		basic_info.have_container = JxlBool::False;

		basic_info.bits_per_sample = 8;
		basic_info.exponent_bits_per_sample = 0;
		basic_info.alpha_premultiplied = JxlBool::False;
		basic_info.alpha_exponent_bits = 0;

		// Adjust for alpha.
		if alpha {
			basic_info.num_extra_channels = 1;
			basic_info.alpha_bits = 8;
		}
		else {
			basic_info.num_extra_channels = 0;
			basic_info.alpha_bits = 0;
		}

		// Decrease the color count if we're working with greyscale. (The
		// default is three.)
		if grey { basic_info.num_color_channels = 1; }

		// Safety: this is an FFI call…
		let color_encoding: JxlColorEncoding = unsafe {
			let mut color_encoding = MaybeUninit::uninit();
			JxlColorEncodingSetToSRGB(
				color_encoding.as_mut_ptr(),
				grey
			);
			color_encoding.assume_init()
		};

		// Safety: this is an FFI call…
		maybe_die(unsafe { JxlEncoderSetBasicInfo(self.0, &basic_info) })?;
		// Safety: this is an FFI call…
		maybe_die(unsafe { JxlEncoderSetColorEncoding(self.0, &color_encoding) })
	}

	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # Write.
	fn write(&self, candidate: &mut Output) -> Result<(), RefractError> {
		// Grab the buffer.
		let buf = candidate.as_mut_vec();

		// Process the output.
		loop {
			let mut len: usize = buf.len();
			let mut avail_out = buf.capacity() - len;

			// Make sure we can write at least 64KiB to the buffer.
			if avail_out < 65_536 {
				buf.try_reserve(65_536).map_err(|_| RefractError::Overflow)?;
				avail_out = buf.capacity() - len;
			}

			// Let JPEG XL do its thing.
			// Safety: this is an FFI call…
			let mut next_out = unsafe { buf.as_mut_ptr().add(len).cast() };
			// Safety: this is an FFI call…
			let res = unsafe {
				JxlEncoderProcessOutput(self.0, &mut next_out, &mut avail_out)
			};

			// Abort on error.
			if res != JxlEncoderStatus::Success && res != JxlEncoderStatus::NeedMoreOutput {
				return Err(RefractError::Encode);
			}

			// The new next offset is how far from the beginning?
			// Safety: this is an FFI call…
			len = usize::try_from(unsafe { next_out.offset_from(buf.as_ptr()) })
				.map_err(|_| RefractError::Overflow)?;

			// Adjust the buffer length to match.
			// Safety: this is an FFI call…
			unsafe { buf.set_len(len); }

			// We're done!
			if JxlEncoderStatus::Success == res { break; }
		}

		Ok(())
	}
}

impl Drop for LibJxlEncoder {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libjxl handles deallocation.
		unsafe { JxlEncoderDestroy(self.0); }
	}
}



/// # Hold the Thread Runner.
///
/// This wrapper exists solely to help with drop cleanup.
struct LibJxlThreadParallelRunner(*mut c_void);

impl LibJxlThreadParallelRunner {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	/// # New instance!
	fn new() -> Result<Self, RefractError> {
		// Safety: this is an FFI call…
		let threads = unsafe {
			JxlThreadParallelRunnerCreate(
				std::ptr::null(),
				std::thread::available_parallelism().map_or(1, NonZeroUsize::get),
			)
		};
		if threads.is_null() { Err(RefractError::Encode) }
		else { Ok(Self(threads)) }
	}
}

impl Drop for LibJxlThreadParallelRunner {
	#[expect(unsafe_code, reason = "Needed for FFI.")]
	#[inline]
	fn drop(&mut self) {
		// Safety: libjxl handles deallocation.
		unsafe { JxlThreadParallelRunnerDestroy(self.0); }
	}
}



#[expect(unsafe_code, reason = "Needed for FFI.")]
/// # Encode.
///
/// This stitches all the pieces together. Who would have thought a
/// convoluted format like JPEG XL would require so many steps to produce?!
fn encode(
	img: &Input,
	candidate: &mut Output,
	quality: Option<NonZeroU8>
) -> Result<(), RefractError> {
	// Initialize the encoder.
	let enc = LibJxlEncoder::new()?;

	// Hook in parallelism.
	let runner = LibJxlThreadParallelRunner::new()?;
	// Safety: this is an FFI call…
	maybe_die(unsafe {
		JxlEncoderSetParallelRunner(
			enc.0,
			JxlThreadParallelRunner,
			runner.0
		)
	})?;

	// Initialize the options wrapper.
	// Safety: this is an FFI call…
	let options: *mut JxlEncoderFrameSettings = unsafe {
		JxlEncoderFrameSettingsCreate(enc.0, std::ptr::null())
	};

	// No containers.
	// Safety: this is an FFI call…
	maybe_die(unsafe { JxlEncoderUseContainer(enc.0, false) })?;

	// Set distance and losslessness.
	let q = match quality.map(NonZeroU8::get) {
		Some(q) if q < 150 => f32::from(150_u8 - q) / 10.0,
		_ => 0.0,
	};
	// Safety: this is an FFI call…
	maybe_die(unsafe { JxlEncoderSetFrameLossless(options, 0.0 == q) })?;
	// Safety: this is an FFI call…
	maybe_die(unsafe { JxlEncoderSetFrameDistance(options, q) })?;

	// Effort. 9 == Tortoise.
	// Safety: this is an FFI call…
	maybe_die(unsafe { JxlEncoderFrameSettingsSetOption(options, JxlEncoderFrameSettingId::Effort, 9) })?;

	// Decoding speed. 0 == Highest quality.
	// Safety: this is an FFI call…
	maybe_die(unsafe { JxlEncoderFrameSettingsSetOption(options, JxlEncoderFrameSettingId::DecodingSpeed, 0) })?;

	// Set up JPEG XL's "basic info" struct.
	let color = img.color();
	enc.set_basic_info(img.width_u32(), img.height_u32(), color.has_alpha(), color.is_greyscale())?;

	// Set up a "frame".
	let pixel_format = JxlPixelFormat {
		num_channels: color.channels(),
		data_type: JxlDataType::Uint8,
		endianness: JxlEndianness::Native,
		align: 0,
	};

	// JXL really fucks up alpha at lower qualities.
	if color.has_alpha() {
		// Safety: this is an FFI call…
		maybe_die(unsafe { JxlEncoderSetExtraChannelDistance(options, 0, 0.0) })?;
	}

	let data: &[u8] = img;
	// Safety: this is an FFI call…
	maybe_die(unsafe {
		JxlEncoderAddImageFrame(
			options,
			&pixel_format,
			data.as_ptr().cast(),
			size_of_val(data),
		)
	})?;

	// Finalize the encoder.
	// Safety: this is an FFI call…
	unsafe { JxlEncoderCloseInput(enc.0); }
	enc.write(candidate)
}

/// # Verify Encoder Status.
///
/// Most `JPEG XL` API methods return a status; this converts unsuccessful
/// statuses to a proper Rust error.
const fn maybe_die(res: JxlEncoderStatus) -> Result<(), RefractError> {
	match res {
		JxlEncoderStatus::Success => Ok(()),
		_ => Err(RefractError::Encode),
	}
}

/// # Verify Decoder Status.
const fn maybe_die_dec(res: JxlDecoderStatus) -> Result<(), RefractError> {
	match res {
		JxlDecoderStatus::Success => Ok(()),
		_ => Err(RefractError::Decode),
	}
}
