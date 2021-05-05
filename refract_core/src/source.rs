/*!
# `Refract` - Source Image
*/

use crate::{
	EncodeIter,
	Image,
	OutputKind,
	RefractError,
};
use std::{
	borrow::Cow,
	convert::TryFrom,
	num::NonZeroU64,
	path::{
		Path,
		PathBuf,
	},
};



#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Source Image Kind.
///
/// A list of supported source image kinds.
pub enum SourceKind {
	/// # `JPEG`.
	Jpeg,
	/// # `PNG`.
	Png,
}

impl TryFrom<&[u8]> for SourceKind {
	type Error = RefractError;

	/// # From Bytes.
	///
	/// Obtain the image kind from the raw file bytes by inspecting its magic
	/// headers.
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



#[derive(Debug, Clone)]
/// # Image Source.
///
/// This holds the path of a source image along with an owned RGBA buffer.
pub struct Source<'a> {
	path: Cow<'a, Path>,
	size: NonZeroU64,
	img: Image<'a>,
	kind: SourceKind,
}

impl<'a> TryFrom<&'a Path> for Source<'a> {
	type Error = RefractError;

	fn try_from(path: &'a Path) -> Result<Self, Self::Error> {
		let raw: &[u8] = &std::fs::read(path).map_err(|_| RefractError::Read)?;
		let kind = SourceKind::try_from(raw)?;

		// We know this is non-zero because we were able to obtain a valid
		// image kind from its headers.
		let size = u64::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;
		let size = unsafe { NonZeroU64::new_unchecked(size) };

		Ok(Self {
			path: Cow::Borrowed(path),
			size,
			img: Image::try_from(raw)?,
			kind,
		})
	}
}

impl TryFrom<PathBuf> for Source<'_> {
	type Error = RefractError;

	fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
		let raw: &[u8] = &std::fs::read(&path).map_err(|_| RefractError::Read)?;
		let kind = SourceKind::try_from(raw)?;

		// We know this is non-zero because we were able to obtain a valid
		// image kind from its headers.
		let size = u64::try_from(raw.len()).map_err(|_| RefractError::Overflow)?;
		let size = unsafe { NonZeroU64::new_unchecked(size) };

		Ok(Self {
			path: Cow::Owned(path),
			size,
			img: Image::try_from(raw)?,
			kind,
		})
	}
}

/// ## Getters.
impl Source<'_> {
	#[must_use]
	/// # Kind.
	///
	/// Return the type of image.
	pub const fn kind(&self) -> SourceKind { self.kind }

	#[must_use]
	/// # Image (reference).
	///
	/// Return a reference to the image buffer.
	pub fn img(&self) -> Image<'_> { self.img.as_ref() }

	#[must_use]
	/// # Compact Image (reference).
	///
	/// Return a compacted version of the image buffer.
	pub fn img_compact(&self) -> Image<'_> { self.img.as_compact() }

	#[must_use]
	/// # YUV Image (reference).
	///
	/// Return an image buffer converted to YUV range, either limited or full
	/// depending on the flag and source colorness.
	pub(crate) fn img_yuv(&self, flags: u8) -> Image<'_> { self.img.as_yuv(flags) }

	#[must_use]
	/// # Path.
	///
	/// Return a reference to the original path.
	pub fn path(&self) -> &Path { self.path.as_ref() }

	#[must_use]
	/// # Size.
	///
	/// Return the file size of the source.
	pub const fn size(&self) -> NonZeroU64 { self.size }

	#[must_use]
	#[inline]
	/// # Can Do YUV?
	///
	/// This is a convenient function that will evaluate whether an image
	/// source supports limited-range YUV encoding.
	pub fn supports_yuv_limited(&self) -> bool { self.img.supports_yuv_limited() }
}

/// ## Encoding.
impl Source<'_> {
	#[inline]
	#[must_use]
	/// # Encode.
	///
	/// This returns a guided encoding iterator. See [`EncodeIter`] for more
	/// information.
	pub fn encode(&self, enc: OutputKind, flags: u8) -> EncodeIter<'_> {
		EncodeIter::new(self, enc, flags)
	}
}



#[cfg(test)]
mod tests {
	use super::*;
	use crate::ColorKind;
	use crate::PixelKind;

	#[test]
	/// Greyscale Image.
	fn t_myrna() {
		let image = std::fs::canonicalize(
			concat!(env!("CARGO_MANIFEST_DIR"), "/../skel/assets/myrna.png")
		).expect("Missing myrna.png test image.");

		let source = Source::try_from(image).expect("Unable to create Source.");
		assert_eq!(source.kind(), SourceKind::Png);

		// Check out the RGBA image.
		{
			let img = source.img();

			let color = img.color_kind();
			assert_eq!(color, ColorKind::Grey);

			let pixel = img.pixel_kind();
			assert_eq!(pixel, PixelKind::Full);

			assert_eq!((&*img).len(), img.width() * img.height() * 4);
		}

		// Check out the compact version.
		{
			let img = source.img_compact();

			let color = img.color_kind();
			assert_eq!(color, ColorKind::Grey);

			let pixel = img.pixel_kind();
			assert_eq!(pixel, PixelKind::Compact);

			assert_eq!((&*img).len(), img.width() * img.height());
		}
	}

	#[test]
	/// RGB Image.
	fn t_cats() {
		let image = std::fs::canonicalize(
			concat!(env!("CARGO_MANIFEST_DIR"), "/../skel/assets/cats.jpg")
		).expect("Missing cats.jpg test image.");

		let source = Source::try_from(image).expect("Unable to create Source.");
		assert_eq!(source.kind(), SourceKind::Jpeg);

		// Check out the RGBA image.
		{
			let img = source.img();

			let color = img.color_kind();
			assert_eq!(color, ColorKind::Rgb);

			let pixel = img.pixel_kind();
			assert_eq!(pixel, PixelKind::Full);

			assert_eq!((&*img).len(), img.width() * img.height() * 4);
		}

		// Check out the compact version.
		{
			let img = source.img_compact();

			let color = img.color_kind();
			assert_eq!(color, ColorKind::Rgb);

			let pixel = img.pixel_kind();
			assert_eq!(pixel, PixelKind::Compact);

			assert_eq!((&*img).len(), img.width() * img.height() * 3);
		}
	}

	#[test]
	/// RGBA Image.
	fn t_r() {
		let image = std::fs::canonicalize(
			concat!(env!("CARGO_MANIFEST_DIR"), "/../skel/assets/r.png")
		).expect("Missing r.png test image.");

		let source = Source::try_from(image).expect("Unable to create Source.");
		assert_eq!(source.kind(), SourceKind::Png);

		// Check out the RGBA image.
		{
			let img = source.img();

			let color = img.color_kind();
			assert_eq!(color, ColorKind::Rgba);

			let pixel = img.pixel_kind();
			assert_eq!(pixel, PixelKind::Full);

			assert_eq!((&*img).len(), img.width() * img.height() * 4);
		}

		// Check out the compact version, which should match the full version.
		{
			let img = source.img();
			let img2 = source.img_compact();

			assert_eq!(&*img, &*img2);
			assert_eq!(img.color_kind(), img2.color_kind());
			assert_eq!(img.pixel_kind(), img2.pixel_kind());
			assert_eq!(img.width(), img2.width());
			assert_eq!(img.height(), img2.height());
			assert_eq!(img.stride(), img2.stride());
		}
	}
}
