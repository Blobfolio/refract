/*!
# `Refract` - CLI Helpers
*/

use dactyl::{
	NicePercent,
	NiceU64,
	NiceU8,
};
use fyi_msg::Msg;
use refract_core::{
	MAX_QUALITY,
	Output,
	OutputKind,
	RefractError,
};
use std::{
	borrow::Cow,
	ffi::OsStr,
	path::Path,
};



/// # Generate Prompt.
///
/// This makes the generic confirmation prompt for a given path.
pub(super) fn path_prompt(path: &Path) -> Msg {
	Msg::plain(format!(
		"Does \x1b[95;1m{}\x1b[0m look good?",
		path.file_name()
			.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy)
	))
		.with_indent(1)
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

/// # Print `OutputKind` Title.
///
/// ```ignore
/// [WebP]
/// ```
pub(super) fn print_outputkind_title(kind: OutputKind) {
	locked_write(&[
		b"\x1b[34m[\x1b[96;1m",
		kind.as_bytes(),
		b"\x1b[0;34m]\x1b[0m\n",
	].concat());
}

/// # Handle Result.
///
/// This will save the result to the specified path if successful. Either way
/// it will print a nice ANSI-formatted summary message.
pub(super) fn handle_result(
	size: u64,
	path: &Path,
	result: Result<Output, RefractError>
) {
	match result {
		Ok(res) => {
			// Write the result to a file. If this fails, recurse and print the
			// error.
			if let Err(e) = res.write(path) {
				return handle_result(size, path, Err(e));
			}

			// Crunch some details.
			let diff = size - res.size().get();
			let per = dactyl::int_div_float(diff, size);
			let name = path.file_name()
				.map_or_else(|| Cow::Borrowed("?"), OsStr::to_string_lossy);

			// Lossless.
			if res.quality() == MAX_QUALITY {
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
					NiceU8::from(res.quality().get()).as_str(),
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
		},
		Err(e) => {
			// Try to remove the output path; we didn't write to it.
			if path.exists() {
				let _res = std::fs::remove_file(path);
			}
			Msg::warning(e.as_str()).with_indent(1).print();
		},
	}
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
