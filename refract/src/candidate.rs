/*!
# Refract: Candidate
*/

use iced::widget::image;
use refract_core::{
	ImageKind,
	Input,
	Output,
	Quality,
	RefractError,
};
use std::num::NonZeroU32;



#[derive(Debug)]
/// # Candidate.
///
/// This holds the decoded pixels and basic details for a newly-converted image
/// for display purposes.
pub(super) struct Candidate {
	/// # Iced-Ready Image Data.
	pub(super) img: image::Handle,

	/// # Kind.
	pub(super) kind: ImageKind,

	/// # Quality.
	pub(super) quality: Quality,

	/// # Iteration Count.
	pub(super) count: u8,
}

impl TryFrom<Input> for Candidate {
	type Error = RefractError;

	/// # Source Image.
	fn try_from(src: Input) -> Result<Self, Self::Error> {
		let src = src.into_rgba();
		let width = u32::try_from(src.width()).ok()
			.and_then(NonZeroU32::new)
			.ok_or(RefractError::Overflow)?;
		let height = u32::try_from(src.height()).ok()
			.and_then(NonZeroU32::new)
			.ok_or(RefractError::Overflow)?;
		let kind = src.kind();

		Ok(Self {
			img: image::Handle::from_rgba(width.get(), height.get(), src.take_pixels()),
			kind,
			quality: Quality::Lossless(kind),
			count: 0,
		})
	}
}

impl TryFrom<&Output> for Candidate {
	type Error = RefractError;

	#[inline]
	/// # Candidate Image.
	fn try_from(src: &Output) -> Result<Self, Self::Error> {
		let quality = src.quality(); // Note the quality.
		let mut out = Input::try_from(src.as_ref()).and_then(Self::try_from)?;
		out.quality = quality;       // Quality goes here.
		Ok(out)
	}
}

impl Candidate {
	/// # With Count.
	///
	/// This method is used to add an iteration count to a [`Candidate`]
	/// created from a raw source (which doesn't have this information).
	pub(super) const fn with_count(mut self, count: u8) -> Self {
		self.count = count;
		self
	}
}
