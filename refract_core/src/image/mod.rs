/*!
# `Refract` - Image Business
*/

mod alpha;
pub(super) mod color;
pub(super) mod pixel;

use crate::{
	ColorKind,
	PixelKind,
	RefractError,
	SourceKind,
};
use std::{
	borrow::{
		Borrow,
		Cow,
	},
	convert::TryFrom,
	num::NonZeroUsize,
	ops::Deref,
};



#[derive(Debug, Clone)]
/// # Image.
///
/// This struct holds image pixel data, which could be anywhere between 1-4
/// channels depending on the pixel type.
///
/// The image buffer is `CoW` so may be owned or borrowed. [`Image::as_ref`]
/// and [`Image::as_compact`] will try to avoid allocation when possible, but
/// some conversions will require creating a new owned instance.
///
/// The underlying buffer can be accessed through `Deref` as an `&[u8]`.
pub struct Image<'a> {
	img: Cow<'a, [u8]>,
	color: ColorKind,
	pixel: PixelKind,
	width: NonZeroUsize,
	height: NonZeroUsize,
}

impl Deref for Image<'_> {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target { self.img.as_ref() }
}

impl TryFrom<&[u8]> for Image<'_> {
	type Error = RefractError;

	#[inline]
	/// # Try From Raw Image.
	///
	/// This will generate an [`Image`] from raw file bytes.
	///
	/// ## Errors
	///
	/// This will return an error if the source is not a valid JPEG or PNG, or
	/// if it uses an unsupported color scheme.
	fn try_from(raw: &[u8]) -> Result<Self, Self::Error> {
		match SourceKind::try_from(raw)? {
			SourceKind::Jpeg => Self::from_jpg(raw),
			SourceKind::Png => Self::from_png(raw),
		}
	}
}

impl Image<'_> {
	/// # Try From JPEG.
	///
	/// This will generate an [`Image`] from a JPEG source (already verified).
	///
	/// ## Errors
	///
	/// This will return an error if the dimensions overflow or other weird
	/// things come up during decoding.
	fn from_jpg(mut raw: &[u8]) -> Result<Self, RefractError> {
		use jpeg_decoder::PixelFormat;
		use rgb::{ComponentSlice, FromSlice};

		// Decode the image.
		let mut jecoder = jpeg_decoder::Decoder::new(&mut raw);
		let pixels = jecoder.decode()
			.map_err(|_| RefractError::Source)?;
		let info = jecoder.info().ok_or(RefractError::Source)?;

		let width: usize = info.width.into();
		let height: usize = info.height.into();

		// So many ways to be a JPEG...
		let (raw, any_color): (Vec<u8>, bool) = match info.pixel_format {
			// Upscale greyscale to RGBA.
			PixelFormat::L8 => (
				pixels.iter()
					.fold(Vec::with_capacity(width * height * 4), |mut acc, &px| {
						acc.extend_from_slice(&[px, px, px, 255]);
						acc
					}),
				false
			),
			// Upscale RGB to RGBA.
			PixelFormat::RGB24 =>  pixels.as_rgb()
				.iter()
				.map(|px| px.alpha(255))
				.fold(
					(Vec::with_capacity(width * height * 4), false), |mut acc, px| {
					acc.0.extend_from_slice(px.as_slice());
					(
						acc.0,
						acc.1 || px.r != px.g || px.r != px.b,
					)
				}),
			// CMYK isn't supported.
			PixelFormat::CMYK32 => return Err(RefractError::Color),
		};

		let color =
			if any_color { ColorKind::Rgb }
			else { ColorKind::Grey };

		// One final sanity check to make sure the buffer is sized correctly!
		if raw.len() != width * height * 4 {
			return Err(RefractError::Overflow);
		}

		Ok(Self {
			img: Cow::Owned(raw),
			color,
			pixel: PixelKind::Full,
			width: NonZeroUsize::new(width).ok_or(RefractError::Overflow)?,
			height: NonZeroUsize::new(height).ok_or(RefractError::Overflow)?,
		})
	}

	/// # Try From PNG.
	///
	/// This will generate an [`Image`] from a PNG source (already verified).
	///
	/// ## Errors
	///
	/// This will return an error if the dimensions overflow or other weird
	/// things come up during decoding.
	fn from_png(raw: &[u8]) -> Result<Self, RefractError> {
		// Grab the RGBA pixels, width, and height.
		let (mut raw, width, height): (Vec<u8>, usize, usize) = {
			// Parse the file.
			let decoder = spng::Decoder::new(raw)
				.with_output_format(spng::Format::Rgba8)
				.with_decode_flags(spng::DecodeFlags::TRANSPARENCY);
			let (info, mut reader) = decoder.read_info()
				.map_err(|_| RefractError::Source)?;

			// Grab the dimensions.
			let width = usize::try_from(info.width)
				.map_err(|_| RefractError::Overflow)?;
			let height = usize::try_from(info.height)
				.map_err(|_| RefractError::Overflow)?;

			// Throw the pixels into a buffer.
			let mut out = Vec::new();
			out.reserve_exact(info.buffer_size);
			unsafe { out.set_len(info.buffer_size); }
			reader.next_frame(&mut out)
				.map_err(|_| RefractError::Source)?;

			// Make sure the buffer is the right RGBA size.
			if out.len() != width * height * 4 {
				return Err(RefractError::Overflow);
			}

			(out, width, height)
		};

		// Figure out the color/alpha situation. We don't want to trust the
		// source color type, as images might have been saved inappropriately.
		let (any_color, any_alpha) = raw.chunks_exact(4)
			.fold((false, false), |(color, alpha), px| {
				(
					color || px[0] != px[1] || px[0] != px[2],
					alpha || px[3] != 255
				)
			});

		// If we have alpha, let's take a quick detour to clean it up.
		if any_alpha {
			alpha::clean_alpha(&mut raw, width, height);
		}

		let color =
			if any_alpha && any_color { ColorKind::Rgba }
			else if any_color { ColorKind::Rgb }
			else if any_alpha { ColorKind::GreyAlpha }
			else { ColorKind::Grey };

		Ok(Self {
			img: Cow::Owned(raw),
			color,
			pixel: PixelKind::Full,
			width: NonZeroUsize::new(width).ok_or(RefractError::Overflow)?,
			height: NonZeroUsize::new(height).ok_or(RefractError::Overflow)?,
		})
	}
}

/// ## Getters.
impl<'a> Image<'a> {
	#[must_use]
	/// # Color Kind.
	///
	/// This returns the type of color found in this image, e.g. RGB,
	/// greyscale, etc.
	pub const fn color_kind(&self) -> ColorKind { self.color }

	#[must_use]
	/// # Height.
	///
	/// Return the image height.
	pub const fn height(&self) -> usize { self.height.get() }

	#[must_use]
	/// # Pixel Kind.
	///
	/// Return the pixel format kind, either [`PixelKind::Full`] or
	/// [`PixelKind::Compact`].
	pub const fn pixel_kind(&self) -> PixelKind { self.pixel }

	#[must_use]
	/// # Width.
	///
	/// Return the image width.
	pub const fn width(&self) -> usize { self.width.get() }
}

/// ## I32 Helpers.
impl Image<'_> {
	/// # Height.
	///
	/// Return the image height.
	///
	/// ## Errors
	///
	/// This will return an error if the `usize` value doesn't fit.
	pub fn height_i32(&self) -> Result<i32, RefractError> {
		i32::try_from(self.height.get())
			.map_err(|_| RefractError::Overflow)
	}

	/// # Width.
	///
	/// Return the image width.
	///
	/// ## Errors
	///
	/// This will return an error if the `usize` value doesn't fit.
	pub fn width_i32(&self) -> Result<i32, RefractError> {
		i32::try_from(self.width.get())
			.map_err(|_| RefractError::Overflow)
	}
}

/// ## U32 Helpers.
impl Image<'_> {
	/// # Height.
	///
	/// Return the image height.
	///
	/// ## Errors
	///
	/// This will return an error if the `usize` value doesn't fit.
	pub fn height_u32(&self) -> Result<u32, RefractError> {
		u32::try_from(self.height.get())
			.map_err(|_| RefractError::Overflow)
	}

	/// # Width.
	///
	/// Return the image width.
	///
	/// ## Errors
	///
	/// This will return an error if the `usize` value doesn't fit.
	pub fn width_u32(&self) -> Result<u32, RefractError> {
		u32::try_from(self.width.get())
			.map_err(|_| RefractError::Overflow)
	}
}

/// ## Conversion.
impl<'a> Image<'a> {
	#[must_use]
	/// # Compact Buffer.
	///
	/// Clone the image with a buffer reduced to only those channels actually
	/// in use.
	///
	/// If this image is already compacted or uses all channels, no additional
	/// allocations are made. If reduction is necessary, an owned buffer is
	/// created.
	///
	/// ## Panics
	///
	/// This method contains a debug assertion to ensure the buffer ends up
	/// the expected size. This shouldn't ever trigger a failure.
	pub fn as_compact(&'a self) -> Self {
		match self.pixel {
			PixelKind::Compact => self.as_ref(),
			PixelKind::Full => {
				let buf: Vec<u8> = match self.color {
					ColorKind::Rgba => return self.as_ref(),
					ColorKind::Grey => self.img.iter().step_by(4).copied().collect(),
					ColorKind::GreyAlpha => self.img.chunks_exact(4).fold(
						Vec::with_capacity(self.width() * self.height() * 2),
						|mut acc, p| {
							acc.extend_from_slice(&p[2..]);
							acc
						}
					),
					ColorKind::Rgb => self.img.chunks_exact(4).fold(
						Vec::with_capacity(self.width() * self.height() * 3),
						|mut acc, p| {
							acc.extend_from_slice(&p[..3]);
							acc
						}
					),
				};

				debug_assert_eq!(
					buf.len(),
					self.width() * self.height() * self.color.channels() as usize
				);

				Self {
					img: Cow::Owned(buf),
					color: self.color,
					pixel: PixelKind::Compact,
					width: self.width,
					height: self.height,
				}
			},
		}
	}

	#[must_use]
	/// # As Reference.
	///
	/// This is essentially a `Copy`, creating a new [`Image`] instance with a
	/// borrowed reference to this instance's buffer.
	pub fn as_ref(&'a self) -> Self {
		Self {
			img: Cow::Borrowed(self.img.borrow()),
			color: self.color,
			pixel: self.pixel,
			width: self.width,
			height: self.height,
		}
	}
}
