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



/// # The raw HTML template.
const HTML: &[u8] = include_bytes!("../skel/index.html");



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

		// Make the template by replacing all the source-related data. The
		// output-related stuff will be changed later.
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
			ac.stream_replace_all(HTML, &mut template, vals)
				.map_err(|_| RefractError::Read)?;

			template.into_boxed_slice()
		};

		Ok(Self {
			template,
			src,
			page: RefCell::new(tempfile::Builder::new()
				.suffix(".html")
				.tempfile()
				.map_err(|_| RefractError::Write)?),
			flags
		})
	}

	/// # Encode!
	pub(crate) fn encode(&self, kind: OutputKind) {
		// Print a header for the encoding type.
		cli::print_header_kind(kind);

		// Loop it.
		let mut first = true;
		let mut guide = self.src.encode(kind, self.flags);
		while guide.advance()
			.and_then(|data| self.save(kind, data).ok())
			.is_some()
		{
			if self.prompt(first, kind) {
				guide.keep();
			}
			else {
				guide.discard();
			}

			first = false;
		}

		// Wrap it up!
		let time = guide.time();
		self.finish(kind, guide.take());

		// Print the timings.
		cli::print_computation_time(time);
	}

	/// # Finish.
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

	/// # Instructions/prompt.
	fn prompt(&self, first: bool, kind: OutputKind) -> bool {
		if first {
			let tmp = self.page.borrow();
			let path = tmp.path().to_string_lossy();
			Msg::plain(format!(
				"Open \x1b[95;1mfile://{}\x1b[0m in a browser that supports {} images.",
				path,
				kind
			))
				.with_indent(1)
				.with_newline(true)
				.print();

			Msg::plain("Does the re-encoded image look good?")
				.with_indent(1)
				.prompt()
		}
		else {
			Msg::plain("Reload the browser page. Does the re-re-encoded image look good?")
				.with_indent(1)
				.prompt()
		}
	}

	/// # Save Page.
	fn save(&self, kind: OutputKind, data: &[u8]) -> Result<(), RefractError> {
		use std::io::Seek;

		// Truncate the file; we want to write from the beginning.
		let mut tmp = self.page.borrow_mut();
		let file = tmp.as_file_mut();
		file.set_len(0).map_err(|_| RefractError::Write)?;
		file.seek(SeekFrom::Start(0)).map_err(|_| RefractError::Write)?;

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
		ac.stream_replace_all(self.template.as_ref(), file, vals)
			.map_err(|_| RefractError::Write)
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
