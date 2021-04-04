/*!
# `Refract`: Image Kind
*/

use crate::RefractError;
use std::convert::TryFrom;
use std::path::PathBuf;



#[derive(Debug, Clone, Copy, PartialEq)]
/// # (Source) Image Kind.
pub enum ImageKind {
	/// # JPEG.
	Jpeg,
	/// # PNG.
	Png,
}

impl TryFrom<&PathBuf> for ImageKind {
	type Error = RefractError;

	fn try_from(file: &PathBuf) -> Result<Self, Self::Error> {
		let res = imghdr::from_file(file)
			.map_err(|_| RefractError::InvalidImage)?;

		match res {
			Some(imghdr::Type::Jpeg) => Ok(Self::Jpeg),
			Some(imghdr::Type::Png) => Ok(Self::Png),
			_ => Err(RefractError::InvalidImage),
		}
	}
}
