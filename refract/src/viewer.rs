/*!
# `Refract` - Image Viewer
*/

use aho_corasick::AhoCorasick;
use dactyl::NiceU64;
use fyi_msg::Msg;
use refract_core::{
	Output,
	OutputKind,
	RefractError,
	Source,
};
use std::{
	borrow::Cow,
	cell::RefCell,
	convert::TryFrom,
	ffi::OsStr,
	io::SeekFrom,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};
use super::cli;
use tempfile::NamedTempFile;



/// # The raw main HTML template.
const MAIN_HTML: &[u8] = include_bytes!("../skel/main.min.html");

/// # The raw pending HTML template.
const PENDING_HTML: &[u8] = include_bytes!("../skel/pending.min.html");



/// # Image Viewer.
pub(super) struct Viewer<'a> {
	template: Box<[u8]>,
	src: Source<'a>,
	page: RefCell<NamedTempFile>,
	flags: u8,
}

impl Viewer<'_> {
	/// # New Instance.
	///
	/// Create a new instance from a given file.
	pub(crate) fn new(path: PathBuf, flags: u8) -> Result<Self, RefractError> {
		let raw: &[u8] = &std::fs::read(&path).map_err(|_| RefractError::Read)?;
		let src = Source::try_from(path)?;

		// To save some effort, let's pre-crunch the template using all of the
		// source-related information (as it won't change). This way we only
		// need to update the output-related details on subsequent iterations.
		let template: Box<[u8]> = {
			let keys = &[
				"%filename%",
				"%width%",
				"%height%",
				"%src.type%",
				"%src.base64%",
				"%src.ext%",
			];

			let path = src.path()
				.file_name()
				.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy);
			let img = src.img();
			let width = NiceU64::from(img.width());
			let height = NiceU64::from(img.height());

			let vals = &[
				path.as_ref(),
				width.as_str(),
				height.as_str(),
				unsafe { std::str::from_utf8_unchecked(src.kind().type_bytes()) },
				&base64::encode(raw),
				unsafe { std::str::from_utf8_unchecked(src.kind().as_bytes()) },
			];

			let mut template: Vec<u8> = Vec::new();
			let ac = AhoCorasick::new(keys);
			ac.stream_replace_all(MAIN_HTML, &mut template, vals)
				.map_err(|_| RefractError::Read)?;

			template.into_boxed_slice()
		};

		let out = Self {
			template,
			src,
			page: RefCell::new(tempfile::Builder::new()
				.suffix(".html")
				.tempfile()
				.map_err(|_| RefractError::Write)?),
			flags
		};

		// Print the generic instructions.
		out.instructions()?;

		Ok(out)
	}

	/// # Encode!
	///
	/// This runs the guided encoding iterator for a given output kind,
	/// resulting in the ideal next-generation image if all goes well!
	pub(crate) fn encode(&self, kind: OutputKind) {
		// Print a header for the encoding type.
		cli::print_header_kind(kind);

		let prompt = Msg::plain("\x1b[2m(Reload the test page.)\x1b[0m Does the re-encoded image look good?")
			.with_indent(1);

		// Loop it.
		let mut guide = self.src.encode(kind, self.flags);
		while guide.advance()
			.and_then(|data| self.save(kind, data).ok())
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
		self.finish(kind, guide.take());

		// Print the timings.
		cli::print_computation_time(time);
	}

	/// # Finish.
	///
	/// This handles printing the summary of the encoding process. If errors
	/// were encountered, it prints those, otherwise it confirms the name of
	/// the new image file and mentions how much space it saved, etc.
	fn finish(&self, kind: OutputKind, result: Result<Output, RefractError>) {
		// Handle results.
		match result {
			Ok(result) => {
				let path = PathBuf::from(OsStr::from_bytes(&[
					self.src.path().as_os_str().as_bytes(),
					kind.ext_bytes()
				].concat()));

				match save_image(&path, &result) {
					Ok(_) => cli::print_success(self.src.size().get(), &result, &path),
					Err(e) => cli::print_error(e),
				}
			},
			Err(e) => cli::print_error(e),
		}
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
		let mut tmp = self.page.borrow_mut();
		{
			let file = tmp.as_file_mut();

			file.write_all(PENDING_HTML)
				.and_then(|_| file.flush())
				.map_err(|_| RefractError::Write)?;
		}

		// Print a message!
		Msg::plain(format!("\
			\n    Open \x1b[92;1mfile://{}\x1b[0m in a web browser, making\
			\n    sure it supports the image format(s) you're encoding.\n\n",
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
		let mut tmp = self.page.borrow_mut();
		let file = tmp.as_file_mut();
		file.set_len(0).map_err(|_| RefractError::Write)?;
		file.seek(SeekFrom::Start(0)).map_err(|_| RefractError::Write)?;

		Ok(())
	}

	/// # Save Page.
	///
	/// This generates and saves the test page for a given candidate image.
	fn save(&self, kind: OutputKind, data: &[u8]) -> Result<(), RefractError> {
		// Make sure we're starting at the beginning of the file.
		self.reset_file()?;

		let keys = &[
			"%ng.type%",
			"%ng.base64%",
			"%ng.ext%",
		];

		let vals = &[
			unsafe { std::str::from_utf8_unchecked(kind.type_bytes()) },
			&base64::encode(data),
			unsafe { std::str::from_utf8_unchecked(kind.as_bytes()) },
		];

		let ac = AhoCorasick::new(keys);
		ac.stream_replace_all(self.template.as_ref(), self.page.borrow_mut().as_file_mut(), vals)
			.map_err(|_| RefractError::Write)
	}
}



/// # Save Image.
///
/// If an acceptable next-generation image has been found, this saves it to a
/// file.
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
