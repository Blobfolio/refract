/*!
# `Refract` - Image CLI
*/

use fyi_msg::Msg;
use refract_core::{
	EncodeIter,
	ImageKind,
	Input,
	Output,
	RefractError,
};
use std::{
	convert::TryFrom,
	path::{
		Path,
		PathBuf,
	},
	time::Duration,
};
use super::utility;



#[derive(Debug)]
/// # Image Source.
pub(super) struct Source<'a> {
	path: PathBuf,
	src: Input<'a>,
	flags: u8,
}

impl Source<'_> {
	/// # New.
	///
	/// Create a new [`Source`] from a file path and flags.
	pub(crate) fn new(path: PathBuf, flags: u8) -> Option<Self> {
		// Print a header regardless of what happens next.
		utility::print_header_path(&path);

		match Self::_new(path, flags) {
			Ok(s) => Some(s),
			Err(e) => {
				utility::print_error(e);
				None
			}
		}
	}

	/// # New Inner.
	///
	/// This method handles actual instantiation, bubbling up any errors it
	/// encounters.
	fn _new(path: PathBuf, flags: u8) -> Result<Self, RefractError> {
		let raw: &[u8] = &std::fs::read(&path).map_err(|_| RefractError::Read)?;
		let src = Input::try_from(raw)?;

		Ok(Self {
			path,
			src,
			flags,
		})
	}

	/// # Encode!
	///
	/// Encode the source to the specified format.
	pub(crate) fn encode(&self, kind: ImageKind) {
		// Print a header for the encoding type.
		utility::print_header_kind(kind);

		// Run the guided encoder.
		let ext: &[u8] = kind.extension().as_bytes();
		let tmp = utility::suffixed_path(&self.path, b".PROPOSED.", ext);
		match self._encode(kind, &tmp) {
			Ok((time, data)) => {
				// Save the permanent version!
				let dst = utility::suffixed_path(&self.path, b".", ext);
				match utility::write_image(&dst, &data) {
					Ok(_) => {
						utility::print_success(self.src.size(), &data, &dst);
						utility::print_computation_time(time);
					},
					Err(e) => utility::print_error(e),
				}
			},
			Err(e) => utility::print_error(e),
		}

		// We can get rid of the temporary file now.
		if tmp.exists() {
			let _res = std::fs::remove_file(tmp);
		}
	}

	/// # Encode Inner.
	///
	/// This handles the actual guided encoding, bubbling up any errors
	/// encountered.
	fn _encode(&self, kind: ImageKind, path: &Path) -> Result<(Duration, Output), RefractError> {
		// We'll be re-using this prompt throughout.
		let prompt = Msg::plain(format!(
			"Does \x1b[95;1m{}\x1b[0m look good?",
			utility::file_name(path)
		))
			.with_indent(1);

		let mut guide = EncodeIter::new(&self.src, kind, self.flags)?;
		while guide.advance()
			.and_then(|data| utility::write_image(path, data).ok())
			.is_some()
		{
			if prompt.prompt() { guide.keep(); }
			else { guide.discard(); }
		}

		let time = guide.time();
		let best = guide.take()?;
		Ok((time, best))
	}
}
