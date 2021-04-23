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

#![allow(clippy::module_name_repetitions)]



mod image;

use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use image::ImageCli;
use refract_core::{
	OutputKind,
	RefractError,
	Source,
};
use dowser::{
	Dowser,
	Extension,
};
use fyi_msg::Msg;
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
		Err(RefractError::Menu(ArgyleError::WantsVersion)) => {
			println!(concat!("Refract v", env!("CARGO_PKG_VERSION")));
		},
		Err(RefractError::Menu(ArgyleError::WantsHelp)) => {
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
fn _main() -> Result<(), RefractError> {
	// The extensions we're going to be looking for.
	const E_JPG: Extension = Extension::new3(*b"jpg");
	const E_PNG: Extension = Extension::new3(*b"png");
	const E_JPEG: Extension = Extension::new4(*b"jpeg");

	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)
		.map_err(RefractError::Menu)?
		.with_list();

	// Figure out which types we're dealing with.
	let mut encoders: Vec<OutputKind> = Vec::with_capacity(2);
	if ! args.switch(b"--no-webp") {
		encoders.push(OutputKind::Webp);
	}
	if ! args.switch(b"--no-avif") {
		encoders.push(OutputKind::Avif);
	}
	if ! args.switch(b"--no-jxl") {
		encoders.push(OutputKind::Jxl);
	}

	if encoders.is_empty() {
		return Err(RefractError::NoEncoders);
	}

	// Find the paths.
	let mut paths = Vec::<PathBuf>::try_from(
		Dowser::filtered(|p|
			Extension::try_from3(p).map_or_else(
				|| Extension::try_from4(p).map_or(false, |e| e == E_JPEG),
				|e| e == E_JPG || e == E_PNG
			)
		)
			.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x.as_ref())))
	)
		.map_err(|_| RefractError::NoImages)?;

	// Sort the paths to make it easier for people to follow.
	paths.sort();

	// Run through the set to see what gets created!
	paths.into_iter()
		.for_each(|x| {
			image::print_path_title(&x);

			match Source::try_from(x) {
				Ok(img) => encoders.iter()
					.map(|&e| ImageCli::new(&img, e))
					.for_each(ImageCli::encode),
				Err(e) => Msg::error(e.as_str()).print(),
			}

			println!();
		});

	Ok(())
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
        --no-jxl      Skip JPEG XL conversion.
        --no-webp     Skip WebP conversion.
    -V, --version     Prints version information.

OPTIONS:
    -l, --list <list> Read image/dir paths from this text file.

ARGS:
    <PATH(S)>...      One or more images or directories to crawl and crunch.
"
	));
}
