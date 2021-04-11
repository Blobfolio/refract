/*!
# `Refract`
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::map_err_ignore)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use dactyl::{
	NiceU64,
	NicePercent,
};
use dowser::Dowser;
use fyi_msg::Msg;
use refract_core::{
	Encoder,
	Image,
	MAX_QUALITY,
	RefractError,
	Refraction,
};
use std::{
	convert::TryFrom,
	ffi::OsStr,
	os::unix::ffi::OsStrExt,
	path::PathBuf,
};



/// # Main.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(ArgyleError::WantsVersion) => {
			println!(concat!("Refract v", env!("CARGO_PKG_VERSION")));
		},
		Err(ArgyleError::WantsHelp) => {
			helper();
		},
		Err(e) => {
			Msg::error(e).die(1);
		},
	}
}

#[inline]
/// # Actual Main.
///
/// This just gives us an easy way to bubble errors up to the real entrypoint.
fn _main() -> Result<(), ArgyleError> {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	// Figure out which types we're dealing with.
	let mut encoders: Vec<Encoder> = Vec::with_capacity(2);
	if ! args.switch(b"--no-webp") {
		encoders.push(Encoder::Webp);
	}
	if ! args.switch(b"--no-avif") {
		encoders.push(Encoder::Avif);
	}

	if encoders.is_empty() {
		return Err(ArgyleError::Custom("With both WebP and AVIF disabled, there is nothing to do!"));
	}

	// Find the paths.
	let mut paths = Vec::<PathBuf>::try_from(
		Dowser::filtered(|p| p.extension()
			.map_or(
				false,
				|e| {
					let ext = e.as_bytes().to_ascii_lowercase();
					ext == b"jpg" || ext == b"png" || ext == b"jpeg"
				}
			)
		)
			.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x.as_ref())))
	)
		.map_err(|_| ArgyleError::Custom("No images were found."))?;

	// Sort the paths to make it easier for people to follow.
	paths.sort();

	// Run through the set to see what gets created!
	paths.iter()
		.for_each(|x|
			if let Ok(img) = Image::try_from(x) {
				img.write_title();

				let size = img.size().get();
				encoders.iter().for_each(|&e| {
					let res = img.try_encode(e);
					print_result(size, res);
				});

				println!();
			}
		);

	Ok(())
}

/// # Print Refraction Result.
fn print_result(size: u64, result: Result<Refraction, RefractError>) {
	match result {
		Ok(res) => {
			let diff = size - res.size().get();
			let per = dactyl::int_div_float(diff, size);

			// Lossless.
			if res.quality() == MAX_QUALITY {
				Msg::success(format!(
					"Created \x1b[1m{}\x1b[0m (lossless).",
					res.name()
				))
			}
			// Lossy.
			else {
				Msg::success(format!(
					"Created \x1b[1m{}\x1b[0m with quality {}.",
					res.name(),
					res.quality()
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
			Msg::warning(e.as_str()).with_indent(1).print();
		},
	}
}

#[cold]
/// # Print Help.
fn helper() {
	println!(concat!(
		r"
             ,,,,,,,,
           ,|||````||||
     ,,,,|||||       ||,
  ,||||```````       `||
,|||`                 |||,
||`     ....,          `|||
||     ::::::::          |||,
||     :::::::'     ||    ``|||,
||,     :::::'               `|||
`||,                           |||
 `|||,       ||          ||    ,||
   `||                        |||`
    ||                   ,,,||||
    ||              ,||||||```
   ,||         ,,|||||`
  ,||`   ||   |||`
 |||`         ||
,||           ||  ", "\x1b[38;5;199mRefract\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r"
||`           ||  Guided WebP/AVIF image conversion
|||,         |||  for JPEG and PNG sources.
 `|||,,    ,|||
   ``||||||||`


USAGE:
    refract [FLAGS] [OPTIONS] <PATH(S)>...

FLAGS:
    -h, --help        Prints help information.
        --no-avif     Skip AVIF conversion.
        --no-webp     Skip WebP conversion.
    -V, --version     Prints version information.

OPTIONS:
    -l, --list <list> Read file paths from this list.

ARGS:
    <PATH(S)>...      One or more images or directories to crawl and crunch.
"
	));
}
