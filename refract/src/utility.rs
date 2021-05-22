/*!
# `Refract` - Cli
*/

use dactyl::{
	NiceElapsed,
	NicePercent,
	NiceU64,
};
use fyi_msg::Msg;
use refract_core::{
	ImageKind,
	Output,
	RefractError,
};
use std::{
	borrow::Cow,
	ffi::OsStr,
	num::NonZeroUsize,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
	time::Duration,
};



#[must_use]
/// # File Name.
///
/// This extracts the file name from a path. If for some reason it doesn't have
/// one, "?" is returned so that _something_ can be printed.
pub(super) fn file_name(path: &Path) -> Cow<str> {
	path.file_name().map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy)
}

/// # Print Output Kind Header.
///
/// This prints the output kind with nice ANSI colors, like:
///
/// ```ignore
/// [AVIF]
/// ```
pub(super) fn print_header_kind(kind: ImageKind) {
	println!("\x1b[34m[\x1b[96;1m{}\x1b[0;34m]\x1b[0m", kind);
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
pub(super) fn print_header_path(path: &Path) {
	let txt = path.to_string_lossy();
	let dashes = "-".repeat(txt.len() + 2);

	println!(
		"\x1b[38;5;199m+{}+\n| \x1b[0m{} \x1b[38;5;199m|\n+{}+\x1b[0m",
		dashes,
		txt,
		dashes,
	);
}

/// # Print Computation Time.
pub(super) fn print_computation_time(time: Duration) {
	Msg::plain(format!(
		"\x1b[2mTotal computation time: {}.\x1b[0m\n",
		NiceElapsed::from(time).as_str(),
	))
		.with_indent(1)
		.print();
}

/// # Print Error.
pub(super) fn print_error(err: RefractError) {
	Msg::warning(err.as_str())
		.with_indent(1)
		.print();
}

/// # Print Success.
pub(super) fn print_success(src_size: usize, output: &Output, dst_path: &Path) {
	let diff: usize = src_size - output.size().map_or(src_size, NonZeroUsize::get);
	let per = dactyl::int_div_float(diff, src_size);

	Msg::success(format!(
		"Created \x1b[1m{}\x1b[0m with {}.",
		file_name(dst_path),
		output.quality(),
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

#[must_use]
/// # Suffixed Path.
///
/// This appends a suffix to a path. It is used to e.g. chuck an extension
/// onto an existing path.
pub(super) fn suffixed_path(path: &Path, glue: &[u8], ext: &[u8]) -> PathBuf {
	PathBuf::from(OsStr::from_bytes(&[
		path.as_os_str().as_bytes(),
		glue,
		ext,
	].concat()))
}

/// # Write Image.
///
/// This saves image data to the specified path.
pub(super) fn write_image(path: &Path, data: &[u8]) -> Result<(), RefractError> {
	use std::io::Write;

	// If the file doesn't exist yet, touch it really quick to set sane
	// starting permissions. (Tempfile doesn't do that.)
	if ! path.exists() {
		std::fs::File::create(path).map_err(|_| RefractError::Write)?;
	}

	tempfile_fast::Sponge::new_for(path)
		.and_then(|mut out| out.write_all(data).and_then(|_| out.commit()))
		.map_err(|_| RefractError::Write)
}
