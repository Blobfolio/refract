/*!
# `Refract` - Source Image
*/

use super::OutputIter;
use super::OutputKind;
use super::RefractError;
use imgref::{
	Img,
	ImgVec,
};
use ravif::RGBA8;
use std::convert::TryFrom;
use std::num::NonZeroU64;
use std::path::PathBuf;



#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Source Kind.
pub enum SourceKind {
	Jpeg,
	Png,
}

impl TryFrom<&[u8]> for SourceKind {
	type Error = RefractError;

	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		match imghdr::from_bytes(src) {
			Some(imghdr::Type::Jpeg) => Ok(Self::Jpeg),
			Some(imghdr::Type::Png) => Ok(Self::Png),
			_ => Err(RefractError::Source),
		}
	}
}



#[derive(Debug)]
/// # Source Image.
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

		Ok(Self {
			path,
			kind: SourceKind::try_from(raw.as_slice())?,
			img: load_rgba(&raw)?,
			size: NonZeroU64::new(u64::try_from(raw.len()).map_err(|_| RefractError::Source)?)
				.ok_or(RefractError::Source)?,
		})
	}
}

/// # Getters.
impl Source {
	#[must_use]
	/// # Image.
	pub fn img(&self) -> Img<&[RGBA8]> { self.img.as_ref() }

	#[must_use]
	/// # Owned Image.
	pub fn img_owned(&self) -> Img<Vec<RGBA8>> { self.img.clone() }

	#[must_use]
	/// # Kind.
	pub const fn kind(&self) -> SourceKind { self.kind }

	#[must_use]
	/// # Path.
	pub const fn path(&self) -> &PathBuf { &self.path }

	#[must_use]
	/// # Size.
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
	pub fn guided_encode(&self, kind: OutputKind) -> OutputIter<'_> {
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
fn load_rgba(mut data: &[u8]) -> Result<ImgVec<RGBA8>, RefractError> {
	use rgb::FromSlice;

	// PNG.
	if data.get(0..4) == Some(&[0x89, b'P', b'N', b'G']) {
		let img = lodepng::decode32(data)
			.map_err(|_| RefractError::Source)?;

		Ok(ImgVec::new(img.buffer, img.width, img.height))
	}
	// JPEG.
	else {
		use jpeg_decoder::PixelFormat::{CMYK32, L8, RGB24};

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
	}
}
