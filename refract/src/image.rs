/*!
# `Refract` - Image CLI
*/

use super::cli;
use fyi_msg::Msg;
use refract_core::{
	Output,
	OutputKind,
	RefractError,
	Source,
};
use std::{
	borrow::Cow,
	ffi::OsStr,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



#[derive(Debug)]
/// # Make Image.
///
/// This struct provides various CLI-related integrations for the guided image
/// conversion process. It is initialized with [`ImageCli::new`] and run with
/// [`ImageCli::encode`].
pub(super) struct ImageCli<'a> {
	src: &'a Source<'a>,
	kind: OutputKind,
	tmp: PathBuf,
	dst: PathBuf,
	flags: u8,
}

impl<'a> Drop for ImageCli<'a> {
	fn drop(&mut self) {
		// Remove the preview file if it still exists.
		if self.tmp.exists() {
			let _res = std::fs::remove_file(&self.tmp);
		}
	}
}

impl<'a> ImageCli<'a> {
	/// # New Instance.
	pub(crate) fn new(src: &'a Source, kind: OutputKind, flags: u8) -> Self {
		// Let's start by setting up the file system paths we'll be using for
		// preview and permanent output.
		let stub: &[u8] = src.path().as_os_str().as_bytes();
		let tmp: PathBuf = PathBuf::from(OsStr::from_bytes(&[stub, b".PROPOSED", kind.ext_bytes()].concat()));
		let dst: PathBuf = PathBuf::from(OsStr::from_bytes(&[stub, kind.ext_bytes()].concat()));

		// We should initialize the tmp path if it doesn't exist to help ensure
		// it has sane permissions; `Sponge` doesn't use a good default.
		if ! tmp.exists() {
			let _res = std::fs::File::create(&tmp);
		}

		Self {
			src,
			kind,
			tmp,
			dst,
			flags,
		}
	}

	/// # Encode.
	pub(crate) fn encode(self) {
		// Print a header for the encoding type.
		cli::print_header_kind(self.kind);

		// We'll be re-using this prompt throughout.
		let prompt = Msg::plain(format!(
			"Does \x1b[95;1m{}\x1b[0m look good?",
			self.tmp.file_name()
				.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy)
		))
			.with_indent(1);

		// Loop it.
		let mut guide = self.src.encode(self.kind, self.flags);
		while guide.advance()
			.and_then(|(data, _)| save_image(&self.tmp, data).ok())
			.is_some()
		{
			if prompt.prompt() {
				guide.keep();
			}
			else {
				guide.discard();
			}
		}

		// Wrap it up!
		let time = guide.time();
		self.finish(guide.take());

		// Print the timings.
		cli::print_computation_time(time);
	}

	/// # Finish.
	fn finish(self, result: Result<Output, RefractError>) {
		// Handle results.
		match result {
			Ok(result) => match save_image(&self.dst, &result) {
				Ok(_) => cli::print_success(self.src.size().get(), &result, &self.dst),
				Err(e) => cli::print_error(e),
			},
			Err(e) => cli::print_error(e),
		}
	}
}



/// # Write Result.
fn save_image(path: &Path, data: &[u8]) -> Result<(), RefractError> {
	use std::io::Write;

	// If the file doesn't exist yet, touch it really quick to set sane
	// starting permissions. (Tempfile doesn't do that.)
	if ! path.exists() {
		std::fs::File::create(path)
			.map_err(|_| RefractError::Write)?;
	}

	tempfile_fast::Sponge::new_for(path)
		.and_then(|mut out| out.write_all(data).and_then(|_| out.commit()))
		.map_err(|_| RefractError::Write)
}
