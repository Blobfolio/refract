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
	/// # Path.
	///
	/// Return a reference to the original path.
	pub fn path(&self) -> &Path { self.path.as_ref() }

	#[must_use]
	/// # Size.
	///
	/// Return the file size of the source.
	pub const fn size(&self) -> NonZeroU64 { self.size }
}

/// ## Encoding.
impl Source<'_> {
	#[inline]
	#[must_use]
	/// # Encode.
	///
	/// This returns a guided encoding iterator. See [`EncodeIter`] for more
	/// information.
	pub fn encode(&self, enc: OutputKind) -> EncodeIter<'_> {
		EncodeIter::new(self, enc)
	}
}
