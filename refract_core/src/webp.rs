/*!
# `Refract`: `WebP` Handling

This uses [`libwebp-sys2`](https://crates.io/crates/libwebp-sys2) bindings to Google's
`libwebp`. Operations should be equivalent to the corresponding `cwebp` output.
*/

use crate::{
	Image,
	ImageKind,
	Quality,
	RefractError,
	Refraction,
};
use fyi_msg::Msg;
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
use ravif::{
	Img,
	RGBA8,
};
use std::{
	convert::TryFrom,
	ffi::OsStr,
	io::Write,
	num::{
		NonZeroU64,
		NonZeroU8,
	},
	os::unix::ffi::OsStrExt,
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # `WebP`.
pub struct Webp<'a> {
	src: Img<&'a [RGBA8]>,
	src_size: NonZeroU64,

	dst: PathBuf,
	dst_size: Option<NonZeroU64>,
	dst_quality: Option<NonZeroU8>,

	tmp: PathBuf,
}

impl<'a> Webp<'a> {
	#[allow(trivial_casts)] // It is what it is.
	#[must_use]
	/// # New.
	///
	/// This instantiates a new instance from an [`Image`] struct. As
	/// [`Webp::find`] is the only other public-facing method, and as it is
	/// consuming, this is generally done as a single chained operation.
	pub fn new(src: &'a Image<'a>) -> Self {
		let stub: &[u8] = unsafe { &*(src.path().as_os_str() as *const OsStr as *const [u8]) };

		let mut out = Self {
			src: src.img(),
			src_size: src.size(),

			dst: PathBuf::from(OsStr::from_bytes(&[stub, b".webp"].concat())),
			dst_size: None,
			dst_quality: None,

			tmp: PathBuf::from(OsStr::from_bytes(&[stub, b".PROPOSED.webp"].concat())),
		};

		// Try lossless while we're here.
		if src.kind() == ImageKind::Png {
			let _res = out.make_lossless();
		}

		out
	}

	/// # Find the best!
	///
	/// This will generate lossy `WebP` image copies in a loop with varying
	/// qualities, asking at each step whether or not the image looks OK. In
	/// most cases, an answer should be found in 5-10 steps.
	///
	/// If an acceptable `WebP` candidate is found — based on user feedback and
	/// file size comparisons — it will be saved as `/path/to/SOURCE.webp`. For
	/// example, if the source lives at `/path/to/image.jpg`, the new version
	/// will live at `/path/to/image.jpg.webp`. In cases where the `WebP` would
	/// be bigger than the source, no image is created.
	///
	/// Note: this method is consuming; the instance will not be usable
	/// afterward.
	///
	/// ## Errors
	///
	/// Returns an error if no acceptable `WebP` can be found or if there are
	/// problems saving them.
	pub fn find(mut self) -> Result<Refraction, RefractError> {
		let prompt = Msg::custom(
			"WebP",
			208,
			&format!(
				"Does \x1b[1;95m{}\x1b[0m look good?",
				self.tmp
					.file_name()
					.ok_or(RefractError::InvalidImage)?
					.to_string_lossy(),
			)
		);

		let mut quality = Quality::default();
		while let Some(q) = quality.next() {
			match self.make_lossy(q) {
				Ok(size) => {
					if prompt.prompt() {
						quality.set_max(q);

						// Move it to the destination.
						std::fs::rename(&self.tmp, &self.dst)
							.map_err(|_| RefractError::Write)?;

						// Update the details.
						self.dst_quality = Some(q);
						self.dst_size = Some(size);
					}
					else {
						quality.set_min(q);
					}
				},
				Err(RefractError::TooBig) => {
					if let Some(x) = NonZeroU8::new(q.get().saturating_sub(1)) {
						quality.set_max(x);
					}
					else { return Err(RefractError::NoWebp); }
				},
				Err(e) => {
					return Err(e);
				},
			}
		}

		// Clean up.
		if self.tmp.exists() {
			let _res = std::fs::remove_file(&self.tmp);
		}

		if let Some((size, quality)) = self.dst_size.zip(self.dst_quality) {
			Ok(Refraction::new(self.dst, size, quality))
		}
		else {
			// Get rid of the distribution file if it exists.
			if self.dst.exists() {
				let _res = std::fs::remove_file(self.dst);
			}

			Err(RefractError::NoWebp)
		}
	}

	/// # Make Lossless.
	///
	/// When the source is a PNG, lossless `WebP` compression will be tried
	/// first.
	///
	/// As "lossless" is more or less lossless, there is no corresponding
	/// prompt. If the resulting file size is smaller than the source, it is
	/// kept.
	///
	/// Afterwards, the program will continue trying lossy compression as
	/// normal.
	fn make_lossless(&mut self) -> Result<(), RefractError> {
		let out = encode(self.src, init_lossless_config())?;

		// What's the size?
	    let size = NonZeroU64::new(u64::try_from(out.len()).map_err(|_| RefractError::Write)?)
			.ok_or(RefractError::Write)?;

		// It has to be smaller than the source.
		if size >= self.src_size {
			return Err(RefractError::TooBig);
		}

		// Save it straight to the destination file; we don't need to preview
		// it since "lossless" should always look right.
		std::fs::File::create(&self.dst)
			.and_then(|mut file| file.write_all(&out).and_then(|_| file.flush()))
			.map_err(|_| RefractError::Write)?;

		// Update the corresponding variables.
		self.dst_size = Some(size);
		self.dst_quality = Some(unsafe { NonZeroU8::new_unchecked(100) });

		Ok(())
	}

	/// # Make Lossy.
	///
	/// Generate a `WebP` image at a given quality size.
	///
	/// ## Errors
	///
	/// This returns an error in cases where the resulting file size is larger
	/// than the source or previous best, or if there are any problems
	/// encountered during encoding or saving.
	fn make_lossy(&self, quality: NonZeroU8) -> Result<NonZeroU64, RefractError> {
		// Clear the temporary file, if any.
		if self.tmp.exists() {
			std::fs::remove_file(&self.tmp).map_err(|_| RefractError::Write)?;
		}

		// How'd it go?
		let out = encode(self.src, init_config(quality))?;

		// What's the size?
	    let size = NonZeroU64::new(u64::try_from(out.len()).map_err(|_| RefractError::Write)?)
			.ok_or(RefractError::Write)?;

		// It has to be smaller than what we've already chosen.
		if let Some(dsize) = self.dst_size {
			if size >= dsize { return Err(RefractError::TooBig); }
		}
		// It has to be smaller than the source.
		else if size >= self.src_size {
			return Err(RefractError::TooBig);
		}

		// Write it to a file!
		std::fs::File::create(&self.tmp)
			.and_then(|mut file| file.write_all(&out).and_then(|_| file.flush()))
			.map_err(|_| RefractError::NoAvif)?;

		Ok(size)
	}
}

/// # Initialize `WebP` Lossy Configuration.
///
/// This generates an encoder configuration profile roughly equivalent to:
///
/// ```bash
/// cwebp -m 6 -pass 10 -q ##
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
/// This converts the raw pixels into a `WebPPicture` object and writer for
/// encoding.
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

	let width = i32::try_from(source.width()).map_err(|_| RefractError::InvalidImage)?;
	let height = i32::try_from(source.height()).map_err(|_| RefractError::InvalidImage)?;

	if width > WEBP_MAX_DIMENSION || height > WEBP_MAX_DIMENSION {
		return Err(RefractError::InvalidImage);
	}

	let mut picture: WebPPicture = unsafe { std::mem::zeroed() };
	if unsafe { WebPPictureInit(&mut picture) } == 0 {
		return Err(RefractError::InvalidImage);
	}

	let argb_stride = i32::try_from(source.stride()).map_err(|_| RefractError::InvalidImage)?;
	picture.use_argb = 1;
	picture.width = width;
	picture.height = height;
	picture.argb_stride = argb_stride;

	// Fill the pixel buffers.
	unsafe {
		use dactyl::traits::SaturatingFrom;
		use rgb::ComponentSlice;

		let mut pixel_data = source
			.pixels()
			.fold(Vec::with_capacity(usize::saturating_from(width * height * 4)), |mut acc, px| {
				acc.extend_from_slice(px.as_slice());
				acc
			});

		let full_stride = argb_stride * 4;

		let status = WebPPictureImportRGBA(
			&mut picture,
			pixel_data.as_mut_ptr(),
			full_stride,
		);

		// A few additional sanity checks.
		let expected_size = argb_stride * height * 4;
		if status == 0 || expected_size == 0 || i32::try_from(pixel_data.len()).unwrap_or(0) != expected_size {
			return Err(RefractError::InvalidImage);
		}

		// Clean-up.
		std::mem::drop(pixel_data);
	}

	// A few more sanity checks.
	if picture.use_argb != 1 || ! picture.y.is_null() || picture.argb.is_null() {
		return Err(RefractError::InvalidImage);
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
		return Err(RefractError::InvalidImage);
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

	if output.is_empty() { Err(RefractError::InvalidImage) }
	else { Ok(output) }
}
