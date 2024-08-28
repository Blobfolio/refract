/*!
# `Refract` - Input Image
*/

use crate::{
	ColorKind,
	ImageKind,
	RefractError,
};
use std::{
	borrow::{
		Borrow,
		Cow,
	},
	fmt,
	num::{
		NonZeroU32,
		NonZeroUsize,
	},
	ops::Deref,
};



#[derive(Clone)]
/// # Input Image.
///
/// This struct holds _decoded_ image data, usually but not always, in the form
/// of a contiguous RGBA (4-byte) slice.
///
/// Both `AsRef<[u8]>` and `Deref` traits are implemented to provide raw access
/// to the pixel slice.
///
/// Other attributes, like dimension and color/depth information, have
/// dedicated getters.
///
/// Similar to `ImgRef`, an [`Input`] can be efficiently borrowed without
/// reallocating the buffers, albeit with lifetime constraints. For a straight
/// copy, use [`Input::borrow`]. To obtain a copy of the image in a different
/// format — RGBA or reduced to only used channels — use [`Input::as_rgba`] or
/// [`Input::as_native`] respectively.
///
/// The latter borrows will require additional allocations if different than
/// the underlying storage, otherwise they are equivalent to [`Input::borrow`].
///
/// Instantiation uses `TryFrom<&[u8]>`, which expects the raw (undecoded) file
/// bytes. At the moment, only `JPEG` and `PNG` image sources can be decoded,
/// but this will likely change with a future release.
///
/// ## Examples
///
/// ```no_run
/// use refract_core::Input;
///
/// let raw = std::fs::read("/path/to/my.jpg").unwrap();
/// let input = Input::try_from(raw.as_slice()).unwrap();
/// ```
pub struct Input<'a> {
	/// # Image Pixels.
	pixels: Cow<'a, [u8]>,

	/// # Image Width.
	width: NonZeroU32,

	/// # Image Height.
	height: NonZeroU32,

	/// # Original File Size.
	size: NonZeroUsize,

	/// # Color Kind.
	color: ColorKind,

	/// Color Depth.
	depth: ColorKind,

	/// # Image Kind.
	kind: ImageKind,
}

impl AsRef<[u8]> for Input<'_> {
	#[inline]
	fn as_ref(&self) -> &[u8] { self }
}

impl fmt::Debug for Input<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Input")
		.field("width", &self.width)
		.field("height", &self.height)
		.field("size", &self.size)
		.field("color", &self.color)
		.field("depth", &self.depth)
		.field("kind", &self.kind)
		.finish()
	}
}

impl Deref for Input<'_> {
	type Target = [u8];

	#[inline]
	fn deref(&self) -> &Self::Target { self.pixels.as_ref() }
}

impl TryFrom<&[u8]> for Input<'_> {
	type Error = RefractError;

	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		let kind = ImageKind::try_from(src)?;
		let (buf, width, height, color) = kind.decode(src)?;

		// Make sure the dimensions are in range.
		let width = u32::try_from(width).ok()
			.and_then(NonZeroU32::new)
			.ok_or(RefractError::Overflow)?;

		let height = u32::try_from(height).ok()
			.and_then(NonZeroU32::new)
			.ok_or(RefractError::Overflow)?;

		// This shouldn't fail since the image decoded, but just in case…
		let size = NonZeroUsize::new(src.len()).ok_or(RefractError::Image)?;

		Ok(Self {
			pixels: Cow::Owned(buf),
			width,
			height,
			size,
			color,
			depth: ColorKind::Rgba,
			kind,
		})
	}
}

/// ## Getters.
impl<'a> Input<'a> {
	#[inline]
	#[must_use]
	/// # Color Kind.
	///
	/// This returns a [`ColorKind`] variant representing the channels actually
	/// used by the image.
	pub const fn color(&self) -> ColorKind { self.color }

	#[inline]
	#[must_use]
	/// # Depth (Pixel Kind).
	///
	/// This returns a [`ColorKind`] variant representing the channels used by
	/// the instance's buffer.
	///
	/// For new objects, storage is always in 4-byte RGBA format, but if
	/// working with an [`Input::as_native`] source, the storage could be any
	/// of `1..=4` bytes per pixel.
	pub const fn depth(&self) -> ColorKind { self.depth }

	#[inline]
	#[must_use]
	/// # Has Alpha?
	///
	/// This returns true if any pixel has an alpha value other than `255`.
	pub const fn has_alpha(&self) -> bool { self.color.has_alpha() }

	#[inline]
	#[must_use]
	/// # Height.
	pub const fn height(&self) -> usize { self.height.get() as usize }

	#[inline]
	#[must_use]
	/// # Is Color?
	///
	/// This returns true if any individual pixel has an R different than its
	/// B or G. For example, `(1, 1, 2)` is color, while `(1, 1, 1)` is not.
	pub const fn is_color(&self) -> bool { self.color.is_color() }

	#[inline]
	#[must_use]
	/// # Is Greyscale?
	///
	/// This returns true if the R, G, and B values of each individual pixel
	/// are equal. For example, `(1, 1, 1)` is greyscale, while `(1, 2, 1)` is
	/// not.
	pub const fn is_greyscale(&self) -> bool { self.color.is_greyscale() }

	#[inline]
	#[must_use]
	/// # Image Kind.
	///
	/// This returns the source image format.
	pub const fn kind(&self) -> ImageKind { self.kind }

	#[inline]
	#[must_use]
	/// # Row Size.
	///
	/// This is equivalent to `width * bytes-per-pixel`. Depending on the
	/// underlying storage, "bytes-per-pixel" can be any of `1..=4`.
	pub const fn row_size(&self) -> usize {
		self.width.get() as usize * self.depth.channels() as usize
	}

	#[inline]
	#[must_use]
	/// # Size.
	///
	/// This returns the size of the original raw image data (the file, not the
	/// pixels).
	pub const fn size(&self) -> usize { self.size.get() }

	#[inline]
	#[must_use]
	/// # Take Pixels.
	///
	/// Consume the instance, stealing the pixels as an owned buffer.
	pub fn take_pixels(self) -> Vec<u8> { self.pixels.into_owned() }

	#[inline]
	#[must_use]
	/// # Width.
	pub const fn width(&self) -> usize { self.width.get() as usize }
}

/// ## I32 Getters.
///
/// These are convenience methods for returning dimensions in `i32` format,
/// which is required by some of the encoders.
impl Input<'_> {
	#[inline]
	/// # Height.
	///
	/// This returns the image height as an `i32`, which is required by some
	/// encoders for whatever reason.
	///
	/// ## Errors
	///
	/// This will return an error if the result does not fit within the `i32`
	/// range.
	pub fn height_i32(&self) -> Result<i32, RefractError> {
		i32::try_from(self.height.get()).map_err(|_| RefractError::Overflow)
	}

	#[inline]
	/// # Row Size.
	///
	/// This is equivalent to `width * bytes-per-pixel`. Depending on the
	/// underlying storage, "bytes-per-pixel" can be any of `1..=4`.
	///
	/// ## Errors
	///
	/// This will return an error if the result does not fit within the `i32`
	/// range.
	pub fn row_size_i32(&self) -> Result<i32, RefractError> {
		i32::try_from(self.row_size()).map_err(|_| RefractError::Overflow)
	}

	#[inline]
	/// # Width.
	///
	/// This returns the image width as an `i32`, which is required by some
	/// encoders for whatever reason.
	///
	/// ## Errors
	///
	/// This will return an error if the result does not fit within the `i32`
	/// range.
	pub fn width_i32(&self) -> Result<i32, RefractError> {
		i32::try_from(self.width.get()).map_err(|_| RefractError::Overflow)
	}
}

/// ## U32 Getters.
///
/// These are convenience methods for returning dimensions in `u32` format,
/// which is required by some of the encoders.
impl Input<'_> {
	#[inline]
	#[must_use]
	/// # Height.
	///
	/// This returns the image height as an `u32`. Because of how [`Input`] is
	/// initialized, the height will always be valid for both `u32` and `usize`
	/// ranges.
	pub const fn height_u32(&self) -> u32 { self.height.get() }

	#[inline]
	#[must_use]
	/// # Width.
	///
	/// This returns the image width as an `u32`. Because of how [`Input`] is
	/// initialized, the width will always be valid for both `u32` and `usize`
	/// ranges.
	pub const fn width_u32(&self) -> u32 { self.width.get() }
}

/// ## Copying and Mutation.
impl<'a> Input<'a> {
	#[must_use]
	/// ## To Native Channels.
	///
	/// Return a copy of the instance holding a buffer reduced to only those
	/// channels actually used by the source. The result may be 1, 2, 3 or 4
	/// bytes.
	///
	/// If the instance is already native, this is equivalent to [`Input::borrow`]
	/// and avoids reallocating the buffer. Otherwise a new owned instance is
	/// returned.
	///
	/// ## Panics
	///
	/// This will panic if called on a non-RGBA source that is also somehow not
	/// the proper native format or if we don't end up with a buffer of the
	/// correct size. Neither of these should be able to happen in practice,
	/// but there is an assertion to make sure.
	pub fn as_native(&'a self) -> Self {
		if self.color == self.depth { return self.borrow(); }
		assert_eq!(self.depth, ColorKind::Rgba);

		let (buf, depth): (Cow<[u8]>, ColorKind) = match self.color {
			ColorKind::Grey => (
				Cow::Owned(
					self.pixels.chunks_exact(4).map(|px| px[0]).collect()
				),
				ColorKind::Grey,
			),
			ColorKind::GreyAlpha => (
				Cow::Owned(
					self.pixels.chunks_exact(4)
					.fold(Vec::with_capacity(self.width() * self.height() * 2), |mut acc, px| {
						acc.push(px[0]); // Keep one color.
						acc.push(px[3]); // Keep alpha.
						acc
					})
				),
				ColorKind::GreyAlpha,
			),
			ColorKind::Rgb => (
				Cow::Owned(
					self.pixels.chunks_exact(4)
					.fold(Vec::with_capacity(self.width() * self.height() * 3), |mut acc, px| {
						acc.extend_from_slice(&px[..3]); // Keep RGB.
						acc
					})
				),
				ColorKind::Rgb,
			),
			// This shouldn't be reachable, but is painless enough to include.
			ColorKind::Rgba => (Cow::Borrowed(self.pixels.borrow()), ColorKind::Rgba),
		};

		assert_eq!(buf.len(), self.width() * self.height() * depth.channels() as usize);

		Self {
			pixels: buf,
			width: self.width,
			height: self.height,
			size: self.size,
			color: self.color,
			depth,
			kind: self.kind,
		}
	}

	#[must_use]
	/// ## To RGBA.
	///
	/// Return a copy of the instance holding a 4-byte RGBA pixel buffer.
	///
	/// If the instance already has an RGBA buffer, this is equivalent to
	/// [`Input::borrow`] and avoids reallocating the buffer. Otherwise a new
	/// owned instance is returned.
	///
	/// ## Panics
	///
	/// This will panic if a 4-byte RGBA slice cannot be created. This
	/// shouldn't happen in practice, but there is an assertion to make sure.
	pub fn as_rgba(&'a self) -> Self {
		// The expected size.
		let size = self.width() * self.height() * 4;

		let buf: Cow<[u8]> = match self.depth {
			ColorKind::Rgba => Cow::Borrowed(self.pixels.borrow()),
			ColorKind::Rgb => Cow::Owned(
				self.pixels.chunks_exact(3)
				.fold(Vec::with_capacity(size), |mut acc, px| {
					acc.extend_from_slice(px); // Push RGB.
					acc.push(255); // Push Alpha.
					acc
				})
			),
			ColorKind::GreyAlpha => Cow::Owned(
				self.pixels.chunks_exact(2)
				.fold(Vec::with_capacity(size), |mut acc, px| {
					acc.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
					acc
				})
			),
			ColorKind::Grey => Cow::Owned(
				self.pixels.iter()
				.copied()
				.fold(Vec::with_capacity(size), |mut acc, px| {
					acc.extend_from_slice(&[px, px, px, 255]);
					acc
				})
			),
		};

		// Make sure we actually filled the buffer appropriately.
		assert_eq!(buf.len(), size);

		Self {
			pixels: buf,
			width: self.width,
			height: self.height,
			size: self.size,
			color: self.color,
			depth: ColorKind::Rgba,
			kind: self.kind,
		}
	}

	#[inline]
	#[must_use]
	/// ## Borrow a Copy.
	///
	/// This will return a new instance using a borrowed pixel buffer (in the
	/// same format as the original), avoiding unnecessary reallocation.
	pub fn borrow(&'a self) -> Self {
		Self {
			pixels: Cow::Borrowed(self.pixels.borrow()),
			width: self.width,
			height: self.height,
			size: self.size,
			color: self.color,
			depth: self.depth,
			kind: self.kind,
		}
	}
}
