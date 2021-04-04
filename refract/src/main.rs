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
	Image,
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
fn _main() -> Result<(), ArgyleError> {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	// Put it all together!
	let paths = Vec::<PathBuf>::try_from(
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

	let webp = ! args.switch(b"--no-webp");
	let avif = ! args.switch(b"--no-avif");

	// Process each.
	paths.iter().for_each(|x| {
		if let Ok(img) = Image::try_from(x) {
			Msg::custom("Source", 199, x.to_string_lossy().as_ref())
				.with_newline(true)
				.print();

			if webp {
				print_result(img.size().get(), img.try_webp());
			}
			if avif {
				print_result(img.size().get(), img.try_avif());
			}

			println!();
		}
	});

	Ok(())
}

/// # Print Refraction Result.
fn print_result(size: u64, result: Result<Refraction, RefractError>) {
	match result {
		Ok(res) => {
			let diff = size - res.size().get();
			let per = dactyl::int_div_float(diff, size);

			Msg::success(format!(
				"Created {} with quality {}.",
				res.name(),
				res.quality()
			))
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
			Msg::warning(e.as_str()).print();
		},
	}
}

#[cold]
/// # Print Help.
fn helper() {
	println!(concat!(
		r"
                  ,.
                 (\(\)
 ,_              ;  o >
  (`-.          /  (_)
  `=(\`-._____/`   |
   `-( /    -=`\   |
 .==`=(  -= = _/   /`--.
(M==M=M==M=M==M==M==M==M)
 \=N=N==N=N==N=N==N=NN=/   ", "\x1b[38;5;199mChannelZ\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r"
  \M==M=M==M=M==M===M=/    Fast, recursive, multi-threaded
   \N=N==N=N==N=NN=N=/     static Brotli and Gzip encoding.
    \M==M==M=M==M==M/
     `-------------'

USAGE:
    channelz [FLAGS] [OPTIONS] <PATH(S)>...

FLAGS:
        --clean       Remove all existing *.gz *.br files before starting.
    -h, --help        Prints help information.
    -p, --progress    Show progress bar while minifying.
    -V, --version     Prints version information.

OPTIONS:
    -l, --list <list>    Read file paths from this list.

ARGS:
    <PATH(S)>...    One or more files or directories to compress.

---

Note: static copies will only be generated for files with these extensions:

    atom; bmp; css; eot; (geo)json; htc; htm(l); ico; ics; js; manifest; md;
    mjs; otf; rdf; rss; svg; ttf; txt; vcard; vcs; vtt; wasm; xhtm(l); xml; xsl
"
	));
}
