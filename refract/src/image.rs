/*!
# `Refract` - Image CLI
*/

use dactyl::{
	NiceElapsed,
	NicePercent,
	NiceU64,
};
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
	pub(crate) fn new(src: &'a Source, kind: OutputKind) -> Self {
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
		}
	}

	/// # Encode.
	pub(crate) fn encode(self) {
		// Print a header for the encoding type.
		println!("\x1b[34m[\x1b[96;1m{}\x1b[0;34m]\x1b[0m", self.kind);

		// We'll be re-using this prompt throughout.
		let prompt = Msg::plain(format!(
			"Does \x1b[95;1m{}\x1b[0m look good?",
			self.tmp.file_name()
				.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy)
		))
			.with_indent(1);

		// Loop it.
		let mut guide = self.src.encode(self.kind);
		while guide.advance()
			.and_then(|data| save_image(&self.tmp, data).ok())
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
		Msg::plain(format!(
			"\x1b[2mTotal computation time: {}.\x1b[0m\n",
			NiceElapsed::from(time).as_str(),
		))
			.with_indent(1)
			.print();
	}

	/// # Finish.
	fn finish(self, result: Result<Output, RefractError>) {
		// Handle results.
		match result {
			Ok(result) => match save_image(&self.dst, &result) {
				Ok(_) => print_success(self.src.size().get(), &result, &self.dst),
				Err(e) => print_error(e),
			},
			Err(e) => {
				if self.dst.exists() {
					let _res = std::fs::remove_file(&self.dst);
				}
				print_error(e)
			}
		}
	}
}



/// # Print Path Title.
///
/// This prints the source image path with a nice ANSI-colored border, like:
///
/// ```ignore
/// +---------------------+
/// | /path/to/source.png |
/// +---------------------+
/// ```
pub(super) fn print_path_title(path: &Path) {
	let txt = path.to_string_lossy();
	let dashes = "-".repeat(txt.len() + 2);

	println!(
		"\x1b[38;5;199m+{}+\n| \x1b[0m{} \x1b[38;5;199m|\n+{}+\x1b[0m",
		dashes,
		txt,
		dashes,
	);
}

/// # Print Error.
fn print_error(err: RefractError) {
	Msg::warning(err.as_str())
		.with_indent(1)
		.print();
}

/// # Print Success.
fn print_success(src_size: u64, output: &Output, dst_path: &Path) {
	let diff: u64 = src_size - output.size().get();
	let per = dactyl::int_div_float(diff, src_size);
	let name = dst_path.file_name()
		.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy);

	Msg::success(format!(
		"Created \x1b[1m{}\x1b[0m with {}.",
		name,
		output.nice_quality(),
	))
		.with_indent(1)
		.with_suffix(
			if let Some(per) = per {
				format!(
					" \x1b[2m(Saved {} bytes, {}.)\x1b[0m",
					NiceU64::from(diff).as_str(),
					NicePercent::from(per).as_str(),
				)
			}
			else {
				format!(
					" \x1b[2m(Saved {} bytes.)\x1b[0m",
					NiceU64::from(diff).as_str(),
				)
			}
		)
		.print();
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
