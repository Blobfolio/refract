/*!
# `Refract`: Image Kind
*/

use crate::RefractError;
use std::convert::TryFrom;



#[derive(Debug, Clone, Copy, PartialEq)]
/// # (Source) Image Kind.
///
/// The kind is determined using the file's magic headers rather than relying
/// on the file having the correct extension.
pub enum ImageKind {
	/// # JPEG.
	Jpeg,
	/// # PNG.
	Png,
}

impl TryFrom<&[u8]> for ImageKind {
	type Error = RefractError;

	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		match imghdr::from_bytes(src) {
			Some(imghdr::Type::Jpeg) => Ok(Self::Jpeg),
			Some(imghdr::Type::Png) => Ok(Self::Png),
			_ => Err(RefractError::InvalidImage),
		}
	}
}
