/*!
# `Refract GTK` - Candidate
*/

use gtk::gdk_pixbuf::{
	Colorspace,
	Pixbuf,
};
use refract_core::{
	ColorKind,
	Input,
	Output,
	Quality,
	RefractError,
};



#[derive(Debug)]
/// # Candidate.
///
/// This is an image "middleware" that can be shared across threads. (Neither
/// `Pixbuf` nor `Input` are willing to make that journey directly.) It holds
/// a buffer of RGBA pixels, the image dimensions, the encoding quality and
/// iteration number — if applicable — and the byte size of the raw image.
pub(super) struct Candidate {
	buf: Box<[u8]>,
	width: i32,
	height: i32,
	row_size: i32,
	pub(super) quality: Quality,
	pub(super) count: u8,
	pub(super) size: usize,
}

impl TryFrom<&Input<'_>> for Candidate {
	type Error = RefractError;

	/// # Source Image.
	fn try_from(src: &Input) -> Result<Self, Self::Error> {
		// Upscale.
		if src.depth() != ColorKind::Rgba {
			return Self::try_from(&src.as_rgba());
		}

		let width = src.width_i32()?;
		let height = src.height_i32()?;
		let row_size = src.row_size_i32()?;

		Ok(Self {
			buf: src.as_ref().to_vec().into_boxed_slice(),
			width,
			height,
			row_size,
			quality: Quality::Lossless(src.kind()),
			count: 0,
			size: src.size(),
		})
	}
}

impl TryFrom<&Output> for Candidate {
	type Error = RefractError;

	/// # Candidate Image.
	fn try_from(src: &Output) -> Result<Self, Self::Error> {
		let input = Input::try_from(src.as_ref())?;
		let width = input.width_i32()?;
		let height = input.height_i32()?;
		let row_size = input.row_size_i32()?;
		let size = input.size();

		Ok(Self {
			buf: input.take_pixels().into_boxed_slice(),
			width,
			height,
			row_size,
			quality: src.quality(),
			count: 1,
			size,
		})
	}
}

impl From<Candidate> for Pixbuf {
	fn from(src: Candidate) -> Self {
		Self::from_mut_slice(
			src.buf,
			Colorspace::Rgb,
			true,
			8,
			src.width,
			src.height,
			src.row_size,
		)
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
