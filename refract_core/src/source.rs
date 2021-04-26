/*!
# `Refract` - Source Image
*/

use crate::{
	ColorKind,
	OutputIter,
	OutputKind,
	RefractError,
};
use imgref::{
	Img,
	ImgExt,
	ImgVec,
};
use rgb::RGBA8;
use std::{
	borrow::Cow,
	convert::TryFrom,
	num::NonZeroU64,
	path::PathBuf,
};



#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Source Kind.
///
/// This is a list of supported input formats.
pub enum SourceKind {
	Jpeg,
	Png,
}

impl TryFrom<&[u8]> for SourceKind {
	type Error = RefractError;

	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		// If the source is big enough for headers, keep going!
		if src.len() > 12 {
			// PNG has just one way to be!
			if src[..8] == [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'] {
				return Ok(Self::Png);
			}

			// JPEG has a lot of different possible headers. They all start and
			// end the same way, but have some differences in the middle.
			if
				src[..3] == [0xFF, 0xD8, 0xFF] &&
				src[src.len() - 2..] == [0xFF, 0xD9] &&
				(
					src[3] == 0xDB ||
					src[3] == 0xEE ||
					(src[3..12] == [0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01]) ||
					(src[3] == 0xE1 && src[6..12] == [b'E', b'x', b'i', b'f', 0x00, 0x00])
				)
			{
				return Ok(Self::Jpeg);
			}
		}

		Err(RefractError::Source)
	}
}



#[derive(Debug)]
/// # Source Image.
///
/// This struct holds the information for a source image. It is instantiated
/// using `TryFrom<PathBuf>`, like:
///
/// ```no_run
/// use refract_core::Source;
/// use std::convert::TryFrom;
/// use std::path::PathBuf;
///
/// let source = Source::try_from(PathBuf::from("/path/to/image.jpg")).unwrap();
/// ```
///
/// The primary use of this struct is its [`Source::encode`] method,
/// which returns an iterator to help find the best encoding.
pub struct Source {
	path: PathBuf,
	size: NonZeroU64,
	img: ImgVec<RGBA8>,
	kind: SourceKind,
}

impl TryFrom<PathBuf> for Source {
	type Error = RefractError;

	fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
		let raw = std::fs::read(&path)
			.map_err(|_| RefractError::Read)?;

		let kind = SourceKind::try_from(raw.as_slice())?;

		Ok(Self {
			path,
			kind,
			img: load_rgba(&raw, kind)?,
			size: NonZeroU64::new(u64::try_from(raw.len()).map_err(|_| RefractError::Source)?)
				.ok_or(RefractError::Source)?,
		})
	}
}

/// # Getters.
impl Source {
	#[must_use]
	/// # Image.
	///
	/// This returns the image pixel data as a reference.
	pub fn img(&self) -> Img<&[RGBA8]> { self.img.as_ref() }

	#[must_use]
	/// # Owned Image.
	///
	/// This returns an owned copy of the image pixel data via cloning. This is
	/// required by AVIF encoding as it works on a modified source (and we
	/// don't want to pollute the authoritative copy).
	pub fn img_owned(&self) -> Img<Vec<RGBA8>> { self.img.clone() }

	#[must_use]
	/// # Kind.
	///
	/// This returns the input kind.
	pub const fn kind(&self) -> SourceKind { self.kind }

	#[must_use]
	/// # Path.
	///
	/// This returns a reference to the source's file path.
	pub const fn path(&self) -> &PathBuf { &self.path }

	#[must_use]
	/// # Size.
	///
	/// This returns the size of the source image.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}

/// # Encoding.
impl Source {
	#[must_use]
	/// # Guided Encoding.
	///
	/// This method returns an iterator that will try to encode the image at
	/// varying qualities, hopefully arriving at the smallest possible
	/// acceptable candidate.
	///
	/// See [`OutputIter`] for more information.
	pub fn encode(&self, kind: OutputKind) -> OutputIter {
		OutputIter::new(self, kind)
	}
}



#[derive(Debug, Clone)]
/// # Treated Source Data.
///
/// This enum allows us to store different kinds of treated image sources for
/// use with the [`TreatedSource`] struct.
enum TreatedSourceKind<'a> {
	/// # A contiguous buffer slice.
	Buffer(Box<[u8]>),
	/// # A vector of pixels.
	Image(Img<Cow<'a, [RGBA8]>>),
}

impl From<Vec<u8>> for TreatedSourceKind<'_> {
	#[inline]
	fn from(src: Vec<u8>) -> Self {
		Self::Buffer(src.into_boxed_slice())
	}
}

impl<'a> From<Img<&'a [RGBA8]>> for TreatedSourceKind<'a> {
	#[inline]
	fn from(src: Img<&'a [RGBA8]>) -> Self {
		Self::Image(src.into())
	}
}

impl From<ImgVec<RGBA8>> for TreatedSourceKind<'_> {
	#[inline]
	fn from(src: ImgVec<RGBA8>) -> Self {
		Self::Image(src.into())
	}
}



#[derive(Debug, Copy, Clone)]
/// # Treatment Kind.
///
/// This enum is only used for initializing a [`TreatedSource`].
pub enum TreatmentKind {
	/// # RGBA.
	///
	/// A buffer with 4-channel RGBA data, regardless of whether all those
	/// channels are used.
	Full,
	/// # Variable.
	///
	/// A buffer containing only the channels used. It could be anywhere from
	/// one byte per pixel (greyscale) or four bytes per pixel (RGBA).
	Compact,
}



#[derive(Debug, Clone)]
/// # Treated Source.
///
/// This is the raw image data, pre-treated, ready to feed to an encoder.
pub struct TreatedSource {
	img: Box<[u8]>,
	width: usize,
	height: usize,
	stride: usize,
	color: ColorKind,
}

/// # Initialization.
impl TreatedSource {
	#[must_use]
	/// # New.
	pub fn new(img: Img<& [RGBA8]>, style: TreatmentKind) -> Self {
		let color = ColorKind::from(img.as_ref());

		Self {
			width: img.width(),
			height: img.height(),
			stride: img.stride(),
			img: match style {
				TreatmentKind::Full => ColorKind::Rgba.to_buf(img),
				TreatmentKind::Compact => color.to_buf(img),
			},
			color,
		}
	}
}

/// # Getters.
impl TreatedSource {
	#[must_use]
	/// # Buffer.
	///
	/// Return the image buffer as a byte slice.
	///
	/// ## Panics
	///
	/// This will panic if the image is not stored as a
	/// [`TreatedSourceKind::Buffer`]. This program doesn't make that mistake,
	/// but if for some reason you're using this as an external library, make
	/// sure you call the right getter for the right type.
	pub const fn buffer(&self) -> &[u8] { & self.img }

	#[must_use]
	/// # Color Kind.
	pub const fn color(&self) -> ColorKind { self.color }

	#[must_use]
	/// # Width.
	pub const fn width(&self) -> usize { self.width }

	#[must_use]
	/// # Height.
	pub const fn height(&self) -> usize { self.height }

	#[must_use]
	/// # Stride.
	pub const fn stride(&self) -> usize { self.stride }
}



/// # Load RGBA.
///
/// This code was more or less stolen from [`cavif`](https://crates.io/crates/cavif).
/// It will attempt to convert the raw image data into an RGBA `ImgVec` object
/// that can be consumed by the encoders.
///
/// The premultiplied/dirty alpha settings from `cavif` have been removed as
/// they are not supported by `refract`. We can also go a little light on type
/// validation here as that was checked previously.
fn load_rgba(mut data: &[u8], kind: SourceKind) -> Result<ImgVec<RGBA8>, RefractError> {
	match kind {
		SourceKind::Png => {
			let img = lodepng::decode32(data)
				.map_err(|_| RefractError::Source)?;

			Ok(ImgVec::new(img.buffer, img.width, img.height))
		},
		SourceKind::Jpeg => {
			use jpeg_decoder::PixelFormat::{CMYK32, L8, RGB24};
			use rgb::FromSlice;

			let mut jecoder = jpeg_decoder::Decoder::new(&mut data);
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
				CMYK32 => return Err(RefractError::Source),
			};

			Ok(ImgVec::new(buf, info.width.into(), info.height.into()))
		},
	}
}
