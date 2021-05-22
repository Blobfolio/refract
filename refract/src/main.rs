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


pub(self) mod utility;
mod source;
mod browser;


use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use refract_core::{
	FLAG_NO_AVIF_YCBCR,
	FLAG_NO_LOSSLESS,
	FLAG_NO_LOSSY,
	ImageKind,
	RefractError,
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

	// We'll get to these in a bit.
	let mut flags: u8 = 0;

	// General encoding modes.
	if args.switch(b"--skip-lossy") {
		flags |= FLAG_NO_LOSSY;
	}
	if args.switch(b"--skip-lossless") {
		flags |= FLAG_NO_LOSSLESS;
	}

	// Make sure the user didn't do something stupid.
	if
		(FLAG_NO_LOSSY == flags & FLAG_NO_LOSSY) &&
		(FLAG_NO_LOSSLESS == flags & FLAG_NO_LOSSLESS)
	{
		return Err(RefractError::NoCompression);
	}

	// Figure out which types we're dealing with.
	let encoders: Box<[ImageKind]> = {
		let mut encoders: Vec<ImageKind> = Vec::with_capacity(3);
		if ! args.switch(b"--no-webp") {
			encoders.push(ImageKind::Webp);
		}
		if ! args.switch(b"--no-avif") {
			encoders.push(ImageKind::Avif);

			// Deal with the AVIF-specific flag while we're here.
			if args.switch(b"--skip-ycbcr") {
				flags |= FLAG_NO_AVIF_YCBCR;
			}
		}
		if ! args.switch(b"--no-jxl") {
			encoders.push(ImageKind::Jxl);
		}
		encoders.into_boxed_slice()
	};

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

	// Save candidate images to a preview web page.
	if args.switch2(b"-b", b"--browser") {
		paths.into_iter()
			.for_each(|p| {
				if let Some(s) = browser::Source::new(p, flags) {
					encoders.iter().for_each(|e| s.encode(*e));
				}
				println!();
			});
	}
	// Save candidate images to disk.
	else {
		paths.into_iter()
			.for_each(|p| {
				if let Some(s) = source::Source::new(p, flags) {
					encoders.iter().for_each(|e| s.encode(*e));
				}
				println!();
			});
	}

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
||`           ||  Guided AVIF/JPEG XL/WebP image
|||,         |||  conversion for JPEG and PNG sources.
 `|||,,    ,|||
   ``||||||||`


USAGE:
    refract [FLAGS] [OPTIONS] <PATH(S)>...

FLAGS:
    -b, --browser       Output an HTML page that can be viewed in a web
                        browser", "\x1b[91;1m*\x1b[0m", r" to preview encoded images. If omitted, preview
                        images will be saved directly, allowing you to view
                        them in the program of your choosing.
    -h, --help          Prints help information.
        --no-avif       Skip AVIF conversion.
        --no-jxl        Skip JPEG XL conversion.
        --no-webp       Skip WebP conversion.
        --skip-lossless Only perform lossy encoding.
        --skip-lossy    Only perform lossless encoding.
        --skip-ycbcr    Only test full-range RGB AVIF encoding (when encoding
                        AVIFs).
    -V, --version       Prints version information.

OPTIONS:
    -l, --list <list>   Read image/dir paths from this text file.

ARGS:
    <PATH(S)>...        One or more images or directories to crawl and crunch.

-----

", "\x1b[91;1m*\x1b[0mVisit \x1b[34mhttps://blobfolio.com/image-test/\x1b[0m", r" to see which next-generation
 image formats are supported by your web browser.
"
	));
}
