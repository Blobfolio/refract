/*!
# `Refract` - Image Viewer
*/

use aho_corasick::AhoCorasick;
use dactyl::NiceU8;
use fyi_msg::Msg;
use refract_core::{
	EncodeIter,
	ImageKind,
	Input,
	Output,
	RefractError,
};
use std::{
	cell::{
		Cell,
		RefCell,
	},
	convert::TryFrom,
	io::SeekFrom,
	path::PathBuf,
	time::Duration,
};
use super::utility;
use tempfile::NamedTempFile;



/// # The raw main HTML template.
const MAIN_HTML: &[u8] = include_bytes!("../skel/main.min.html");

/// # The raw pending HTML template.
const PENDING_HTML: &[u8] = include_bytes!("../skel/pending.min.html");



#[derive(Debug)]
/// # Image Source.
pub(super) struct Source<'a> {
	path: PathBuf,
	src: Input<'a>,
	tmp: RefCell<NamedTempFile>,
	template: Box<[u8]>,
	count: Cell<u8>,
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

		// A file we can write the web page to.
		let tmp = tempfile::Builder::new()
			.suffix(".html")
			.tempfile()
			.map_err(|_| RefractError::Write)?;

		// Set up the template by copying all of the source-specific data over.
		// This way later encoding runs need only worry about the output-
		// specific details.
		let template = {
			let keys = &[
				"%filename%",
				"%width%",
				"%height%",
				"%src.type%",
				"%src.base64%",
				"%src.ext%",
			];

			let filename = utility::file_name(&path);
			let width = src.width().to_string();
			let height = src.height().to_string();

			let vals = &[
				filename.as_ref(),
				&width,
				&height,
				src.kind().mime(),
				&base64::encode(raw),
				src.kind().as_str(),
			];

			let mut template: Vec<u8> = Vec::new();
			let ac = AhoCorasick::new(keys);
			ac.stream_replace_all(MAIN_HTML, &mut template, vals)
				.map_err(|_| RefractError::Read)?;

			template.into_boxed_slice()
		};

		let out = Self {
			path,
			src,
			tmp: RefCell::new(tmp),
			template,
			count: Cell::new(0),
			flags,
		};

		// Print instructions.
		out.instructions()?;

		Ok(out)
	}

	/// # Encode!
	///
	/// Encode the source to the specified format.
	pub(crate) fn encode(&self, kind: ImageKind) {
		// Print a header for the encoding type.
		utility::print_header_kind(kind);

		match self._encode(kind) {
			Ok((time, data)) => {
				// Save the permanent version!
				let dst = utility::suffixed_path(&self.path, b".", kind.extension().as_bytes());
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
	}

	/// # Encode Inner.
	///
	/// This handles the actual guided encoding, bubbling up any errors
	/// encountered.
	fn _encode(&self, kind: ImageKind) -> Result<(Duration, Output), RefractError> {
		// We'll be re-using this prompt throughout.
		let prompt = Msg::plain("\x1b[2m(Reload the test page.)\x1b[0m Does the re-encoded image look good?")
			.with_indent(1);

		// Reset the count to zero.
		self.count.replace(0);

		let mut guide = EncodeIter::new(&self.src, kind, self.flags)?;
		while guide.advance()
			.and_then(|data| self.save_page(data).ok())
			.is_some()
		{
			if prompt.prompt() { guide.keep(); }
			else { guide.discard(); }
		}

		let time = guide.time();
		let best = guide.take()?;
		Ok((time, best))
	}

	/// # Instructions.
	///
	/// This prints generic operating instructions, namely mentioning the path
	/// to the test HTML document.
	///
	/// The HTML file is re-used for all encodings for a given source, so this
	/// only run once (as part of [`Viewer::new`]).
	fn instructions(&self) -> Result<(), RefractError> {
		use std::io::Write;

		// Make sure we're starting at the beginning of the file.
		self.reset_file()?;

		// Save the browser support page.
		let mut tmp = self.tmp.borrow_mut();
		{
			let file = tmp.as_file_mut();
			file.write_all(PENDING_HTML)
				.and_then(|_| file.flush())
				.map_err(|_| RefractError::Write)?;
		}

		// Print a message!
		Msg::plain(format!("\
			\n    Open \x1b[92;1mfile://{}\x1b[0m in a web browser, making\
			\n    sure it supports the image format(s) you're encoding to.\n\n",
			tmp.path().to_string_lossy(),
		))
			.print();

		Ok(())
	}

	/// # Reset File.
	///
	/// This truncates the file and resets its cursor to ensure we are always
	/// starting from scratch when writing the test page. (We're re-using the
	/// same file for each write.)
	fn reset_file(&self) -> Result<(), RefractError> {
		use std::io::Seek;

		// Truncate the file; we want to write from the beginning.
		let mut tmp = self.tmp.borrow_mut();
		let file = tmp.as_file_mut();
		file.set_len(0).map_err(|_| RefractError::Write)?;
		file.seek(SeekFrom::Start(0)).map_err(|_| RefractError::Write)?;

		Ok(())
	}

	/// # Save Page.
	///
	/// This generates and saves the test page for a given candidate image.
	fn save_page(&self, data: &Output) -> Result<(), RefractError> {
		// Make sure we're starting at the beginning of the file.
		self.reset_file()?;

		let kind = data.kind();

		let keys = &[
			"%ng.label%",
			"%ng.quality%",
			"%ng.type%",
			"%ng.base64%",
			"%ng.ext%",
			"%count%",
		];

		// Increment the count.
		let count = self.count.get() + 1;
		self.count.replace(count);

		let count = NiceU8::from(count);
		let quality = data.quality();
		let q = quality.quality().to_string();
		let vals = &[
			quality.label(),
			&q,
			kind.mime(),
			&base64::encode(data),
			kind.as_str(),
			count.as_str(),
		];

		let ac = AhoCorasick::new(keys);
		ac.stream_replace_all(
			self.template.as_ref(),
			self.tmp.borrow_mut().as_file_mut(),
			vals
		)
			.map_err(|_| RefractError::Write)
	}
}
