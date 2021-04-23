/*!
# `Refract` - Source Image
*/

use crate::{
	OutputIter,
	OutputKind,
	RefractError,
};
use imgref::{
	Img,
	ImgVec,
};
use rgb::RGBA8;
use std::{
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
	pub fn encode(&self, kind: OutputKind) -> OutputIter<'_> {
		OutputIter::new(self, kind)
	}
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
