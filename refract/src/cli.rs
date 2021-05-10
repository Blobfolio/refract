/*!
# `Refract` - Cli
*/

use dactyl::NiceElapsed;
use dactyl::NicePercent;
use dactyl::NiceU64;
use fyi_msg::Msg;
use refract_core::Output;
use refract_core::OutputKind;
use refract_core::RefractError;
use std::borrow::Cow;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;



/// # Print Output Kind Header.
///
/// This prints the output kind with nice ANSI colors, like:
///
/// ```ignore
/// [AVIF]
/// ```
pub(super) fn print_header_kind(kind: OutputKind) {
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
pub(super) fn print_success(src_size: u64, output: &Output, dst_path: &Path) {
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
