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
use imgref::ImgVec;
use rgb::RGBA8;
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
	stride: NonZeroUsize,
}

impl Deref for Image<'_> {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target { self.img.as_ref() }
}

impl TryFrom<&[u8]> for Image<'_> {
	type Error = RefractError;

	/// # Try From Raw Image.
	///
	/// This will generate an [`Image`] from raw file bytes. If you already
	/// a `ImgVec` copy, try from that instead as it will save a step.
	///
	/// ## Errors
	///
	/// This will return an error if the source is not a valid JPEG or PNG, or
	/// if it uses an unsupported color scheme.
	fn try_from(mut raw: &[u8]) -> Result<Self, Self::Error> {
		let kind = SourceKind::try_from(raw)?;

		// Parse the image into an `ImgVec` for consistency.
		let img: ImgVec<RGBA8> = match kind {
			SourceKind::Png => {
				let img = lodepng::decode32(raw)
					.map_err(|_| RefractError::Source)?;
				ImgVec::new(img.buffer, img.width, img.height)
			},
			SourceKind::Jpeg => {
				use jpeg_decoder::PixelFormat::{CMYK32, L8, RGB24};
				use rgb::FromSlice;

				let mut jecoder = jpeg_decoder::Decoder::new(&mut raw);
				let pixels = jecoder.decode()
					.map_err(|_| RefractError::Source)?;
				let info = jecoder.info().ok_or(RefractError::Source)?;

				// So many ways to be a JPEG...
				let buf: Vec<_> = match info.pixel_format {
					// Upscale greyscale to RGBA.
					L8 => {
						pixels.iter().copied().map(|g| RGBA8::new(g, g, g, 255)).collect()
					},
					// Upscale RGB to RGBA.
					RGB24 => {
						let rgb = pixels.as_rgb();
						rgb.iter().map(|p| p.alpha(255)).collect()
					},
					// CMYK doesn't work.
					CMYK32 => return Err(RefractError::Color),
				};

				ImgVec::new(buf, info.width.into(), info.height.into())
			},
		};

		// Finish with `TryFrom<ImgVec>`.
		Self::try_from(img)
	}
}

impl TryFrom<ImgVec<RGBA8>> for Image<'_> {
	type Error = RefractError;

	/// # Try From `ImgVec`.
	///
	/// This will generate an [`Image`] from an `ImgVec`. This will attempt to
	/// clear the alpha data before importing to improve encoding performance
	/// down the road.
	///
	/// ## Errors
	///
	/// This will return an error if the dimensions do not fit the
	/// `NonZeroUsize` range, otherwise it should be OK.
	fn try_from(raw: ImgVec<RGBA8>) -> Result<Self, Self::Error> {
		use rgb::ComponentSlice;

		let width: usize = raw.width();
		let height: usize = raw.height();
		let stride = NonZeroUsize::new(raw.stride()).ok_or(RefractError::Overflow)?;

		// Build up the pixels and figure out the color scheme.
		let mut any_color: bool = false;
		let mut any_alpha: bool = false;
		let img: Vec<u8> = {
			let raw = alpha::clear_alpha(raw);
			raw.pixels().fold(Vec::with_capacity(width * height * 4), |mut acc, p| {
				if p.a != 255 { any_alpha = true; }
				if ! any_color && (p.r != p.g || p.r != p.b) { any_color = true; }
				acc.extend_from_slice(p.as_slice());
				acc
			})
		};

		let color =
			if any_alpha && any_color { ColorKind::Rgba }
			else if any_color { ColorKind::Rgb }
			else if any_alpha { ColorKind::GreyAlpha }
			else { ColorKind::Grey };

		Ok(Self {
			img: Cow::Owned(img),
			color,
			pixel: PixelKind::Full,
			width: NonZeroUsize::new(width).ok_or(RefractError::Overflow)?,
			height: NonZeroUsize::new(height).ok_or(RefractError::Overflow)?,
			stride,
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
	/// # Stride.
	///
	/// Return the image stride.
	pub const fn stride(&self) -> usize { self.stride.get() }

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

	/// # Stride.
	///
	/// Return the image stride.
	///
	/// ## Errors
	///
	/// This will return an error if the `usize` value doesn't fit.
	pub fn stride_i32(&self) -> Result<i32, RefractError> {
		i32::try_from(self.stride.get())
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

	/// # Stride.
	///
	/// Return the image stride.
	///
	/// ## Errors
	///
	/// This will return an error if the `usize` value doesn't fit.
	pub fn stride_u32(&self) -> Result<u32, RefractError> {
		u32::try_from(self.stride.get())
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
	pub fn as_compact(&'a self) -> Self {
		match self.pixel {
			PixelKind::Compact | PixelKind::Yuv => self.as_ref(),
			PixelKind::Full => {
				let buf: Vec<u8> = match self.color {
					ColorKind::Rgba => return self.as_ref(),
					ColorKind::Grey => self.img.iter().step_by(4).copied().collect(),
					ColorKind::GreyAlpha => self.img.chunks_exact(4).fold(
						Vec::with_capacity(self.width() * self.height() * 2),
						|mut acc, p| {
							acc.extend_from_slice(&[p[0], p[3]]);
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

				Self {
					img: Cow::Owned(buf),
					color: self.color,
					pixel: PixelKind::Compact,
					width: self.width,
					height: self.height,
					stride: self.stride,
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
			stride: self.stride,
		}
	}
}

/// # YUV.
impl<'a> Image<'a> {
	#[allow(clippy::cast_possible_truncation)] // Values are clamped.
	#[allow(clippy::cast_sign_loss)] // Values are clamped.
	#[allow(clippy::many_single_char_names)] // Judgey!
	#[must_use]
	/// # As YUV.
	///
	/// This converts a [`PixelKind::Full`] RGBA image into a YUV one.
	///
	/// The internal buffer is filled with all the Ys first, then the Us, then
	/// the Vs, and finally the As.
	///
	/// This is only used for AVIF encoding and because of its specificity, is
	/// only exposed to this crate. (It would be too easy to misuse elsewhere.)
	pub(crate) fn as_yuv(&'a self) -> Self {
		debug_assert!(self.pixel == PixelKind::Full, "Invalid pixel format.");

		let size = self.width.get() * self.height.get();

		let mut y_plane: Vec<u8> = Vec::with_capacity(size);
		let mut u_plane: Vec<u8> = Vec::with_capacity(size);
		let mut v_plane: Vec<u8> = Vec::with_capacity(size);
		let mut a_plane: Vec<u8> = Vec::with_capacity(size);

		self.img.chunks_exact(4).for_each(|rgba| {
			let r = f32::from(rgba[0]);
			let g = f32::from(rgba[1]);
			let b = f32::from(rgba[2]);

			let y  = r.mul_add(0.2126, g.mul_add(0.7152, 0.0722 * b));
			let cb = (b - y) * (0.5 / (1.0 - 0.0722));
			let cr = (r - y) * (0.5 / (1.0 - 0.2126));

			y_plane.push((y * (235.0 - 16.0) / 255.0 + 16.0).round().max(0.0).min(255.0) as u8);
			u_plane.push(((cb + 128.0) * (240.0 - 16.0) / 255.0 + 16.0).round().max(0.0).min(255.0) as u8);
			v_plane.push(((cr + 128.0) * (240.0 - 16.0) / 255.0 + 16.0).round().max(0.0).min(255.0) as u8);
			a_plane.push(rgba[3]);
		});

		// Take over the y_plane and add the rest of the data to it.
		y_plane.append(&mut u_plane);
		y_plane.append(&mut v_plane);
		y_plane.append(&mut a_plane);

		debug_assert!(y_plane.len() == size * 4, "Invalid buffer size.");

		Self {
			img: Cow::Owned(y_plane),
			color: self.color,
			pixel: PixelKind::Yuv,
			width: self.width,
			height: self.height,
			stride: self.stride,
		}
	}

	/// # YUV Plane Pointers.
	///
	/// Return pointers and sizes for YUV/alpha data for AVIF encoding.
	///
	/// This method only applies for images with pixel type [`PixelKind::Yuv`].
	///
	/// This is only used for AVIF encoding and because of its specificity, is
	/// only exposed to this crate. (It would be too easy to misuse elsewhere.)
	///
	/// ## Safety
	///
	/// This method itself is safe, but returns mutable pointers that if
	/// misused would cause trouble.
	pub(crate) unsafe fn yuv(&'a self) -> ([*mut u8; 3], [u32; 3], *mut u8, u32) {
		debug_assert!(self.pixel == PixelKind::Yuv, "Invalid pixel format.");

		let size = self.width.get() * self.height.get();

		// Note: these pixels aren't really mutated.
		let ptr = self.img.as_ptr();
		let yuv_ptr = [
			ptr as *mut u8,
			ptr.add(size) as *mut u8,
			ptr.add(size * 2) as *mut u8,
		];

		let a_ptr = ptr.add(size * 3) as *mut u8;

		// This won't fail because width fits in i32.
		let width32 = self.width_u32().unwrap();

		(
			yuv_ptr,
			[width32, width32, width32],
			a_ptr,
			if self.color.has_alpha() { width32 }
			else { 0 }
		)
	}
}
