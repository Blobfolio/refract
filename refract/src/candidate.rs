/*!
# Refract: Candidate
*/

use refract_core::{
	ColorKind,
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
	/// # Image Data.
	pub(super) buf: Box<[u8]>,

	/// # Image Width.
	pub(super) width: NonZeroU32,

	/// # Image Height.
	pub(super) height: NonZeroU32,

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
		Self::try_from(&src)
	}
}

impl TryFrom<&Input> for Candidate {
	type Error = RefractError;

	/// # Source Image.
	fn try_from(src: &Input) -> Result<Self, Self::Error> {
		// Upscale.
		if src.depth() != ColorKind::Rgba {
			return Self::try_from(src.clone().into_rgba());
		}

		let width = u32::try_from(src.width()).ok()
			.and_then(NonZeroU32::new)
			.ok_or(RefractError::Overflow)?;
		let height = u32::try_from(src.height()).ok()
			.and_then(NonZeroU32::new)
			.ok_or(RefractError::Overflow)?;
		let kind = src.kind();

		Ok(Self {
			buf: Box::from(src.as_ref()),
			width,
			height,
			kind,
			quality: Quality::Lossless(src.kind()),
			count: 0,
		})
	}
}

impl TryFrom<&Output> for Candidate {
	type Error = RefractError;

	#[inline]
	/// # Candidate Image.
	fn try_from(src: &Output) -> Result<Self, Self::Error> {
		let quality = src.quality();
		let mut out = Input::try_from(src.as_ref()).and_then(|i| Self::try_from(&i))?;
		out.quality = quality;
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
