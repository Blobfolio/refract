/*!
# `Refract` - Image CLI
*/

use dactyl::{
	NicePercent,
	NiceU64,
	NiceU8,
};
use fyi_msg::Msg;
use refract_core::{
	OutputIter,
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
	src: &'a Source,
	kind: OutputKind,
	guide: OutputIter<'a>,
	tmp: PathBuf,
	dst: PathBuf,
}

impl<'a> ImageCli<'a> {
	/// # Print Path Title.
	///
	/// This prints the source image path with a nice ANSI-colored border, like:
	///
	/// ```ignore
	/// +---------------------+
	/// | /path/to/source.png |
	/// +---------------------+
	/// ```
	pub(crate) fn print_path_title(path: &Path) {
		let txt = path.to_string_lossy();
		let dashes = "-".repeat(txt.len() + 2);

		locked_write(&[
			b"\x1b[38;5;199m+",
			dashes.as_bytes(),
			b"+\n| \x1b[0m",
			txt.as_bytes(),
			b" \x1b[38;5;199m|\n+",
			dashes.as_bytes(),
			b"+\x1b[0m\n",
		].concat());
	}

	#[inline]
	/// # Print line break.
	pub(crate) fn print_newline() { locked_write(b"\n"); }

	#[allow(trivial_casts)] // Triviality is necessary.
	/// # New Instance.
	pub(crate) fn new(src: &'a Source, kind: OutputKind) -> Self {
		// Let's start by setting up the file system paths we'll be using for
		// preview and permanent output.
		let stub: &[u8] = unsafe { &*(src.path().as_os_str() as *const OsStr as *const [u8]) };
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
			guide: src.encode(kind),
			tmp,
			dst,
		}
	}

	/// # Encode.
	pub(crate) fn encode(mut self) {
		// Print a header for the encoding type.
		locked_write(&[
			b"\x1b[34m[\x1b[96;1m",
			self.kind.as_bytes(),
			b"\x1b[0;34m]\x1b[0m\n",
		].concat());

		// We'll be re-using this prompt throughout.
		let prompt = Msg::plain(format!(
			"Does \x1b[95;1m{}\x1b[0m look good?",
			self.tmp.file_name()
				.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy)
		))
			.with_indent(1);

		// Loop it.
		while let Some(candidate) = self.guide.next().filter(|c| c.write(&self.tmp).is_ok()) {
			if prompt.prompt() {
				self.guide.keep(candidate);
			}
			else {
				self.guide.discard(candidate);
			}
		}

		// Wrap it up!
		self.finish();
	}

	/// # Finish.
	fn finish(self) {
		// Remove the preview file if it still exists.
		if self.tmp.exists() {
			let _res = std::fs::remove_file(&self.tmp);
		}

		// Handle results.
		match self.guide.take() {
			Ok(res) => match res.write(&self.dst) {
				Ok(_) => print_success(
					self.src.size().get(),
					res.size().get(),
					res.quality().get(),
					&self.dst
				),
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



/// # Print Error.
fn print_error(err: RefractError) {
	Msg::warning(err.as_str())
		.with_indent(1)
		.print();
}

/// # Print Success.
fn print_success(src_size: u64, dst_size: u64, dst_quality: u8, dst_path: &Path) {
	let diff: u64 = src_size - dst_size;
	let per = dactyl::int_div_float(diff, src_size);
	let name = dst_path.file_name()
		.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy);

	// Lossless.
	if dst_quality == 100 {
		Msg::success(format!(
			"Created \x1b[1m{}\x1b[0m (lossless).",
			name
		))
	}
	// Lossy.
	else {
		Msg::success(format!(
			"Created \x1b[1m{}\x1b[0m with quality {}.",
			name,
			NiceU8::from(dst_quality).as_str(),
		))
	}
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

/// # Locked write.
///
/// This prints arbitrary bytes to STDOUT, ensuring the writer is locked and
/// flushed. Errors are suppressed silently.
fn locked_write(data: &[u8]) {
	use std::io::Write;
	let writer = std::io::stdout();
	let mut handle = writer.lock();
	let _res = handle.write_all(data).and_then(|_| handle.flush());
}
