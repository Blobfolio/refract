/*!
# `Refract` - Error
*/

use crate::ImageKind;
use std::{
	error::Error,
	fmt,
};



#[cfg(feature = "bin")]
/// # Help Text.
const HELP: &str = concat!(r"
       ..oFaa7l;'
   =>r??\O@@@@QNk;
  :|Fjjug@@@@@@@@N}}:
 ^/aPePN@@@@peWQ@Qez;
 =iKBDB@@@O^:.::\kQO=~
 =iKQ@QWOP: ~gBQw'|Qgz,
 =i6RwEQ#s' N@RQQl i@D:   ", "\x1b[38;5;199mRefract\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r#"
 =?|>a@@Nv'^Q@@@Qe ,aW|   Guided image conversion from
 ==;.\QQ@6,|Q@@@@p.;;+\,  JPEG/PNG to AVIF/JPEG-XL/WebP.
 '\tlFw9Wgs~W@@@@S   ,;'
 .^|QQp6D6t^iDRo;
   ~b@BEwDEu|:::
    rR@Q6t7|=='
     'i6Ko\=;
       `''''`

USAGE:
    refract [FLAGS] [OPTIONS] <PATH(S)>...

FORMAT FLAGS:
        --no-avif     Skip AVIF encoding.
        --no-jxl      Skip JPEG-XL encoding.
        --no-webp     Skip WebP encoding.

MODE FLAGS:
        --no-lossless Skip lossless encoding passes.
        --no-lossy    Skip lossy encoding passes.
        --no-ycbcr    Skip AVIF YCbCr encoding passes.

MISC FLAGS:
    -h, --help        Print help information and exit.
    -V, --version     Print version information and exit.

OPTIONS:
    -l, --list <FILE> Read (absolute) image and/or directory paths from this
                      text file — or STDIN if "-" — one path per line, instead
                      of or in addition to those specified inline via
                      <PATH(S)>.

TRAILING ARGS:
    <PATH(S)>...      Image and/or directory paths to re-encode. Directories
                      will be crawled recursively.
"#);



#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Errors.
pub enum RefractError {
	/// # Unsupported color.
	Color,

	/// # Decoding failed.
	Decode,

	/// # Encoding failed.
	Encode,

	/// # Invalid image.
	Image,

	/// # Decoding not supported.
	ImageDecode(ImageKind),

	/// # Encoding not supported.
	ImageEncode(ImageKind),

	/// # No candiates were found.
	NoBest(ImageKind),

	/// # Done!
	NothingDoing,

	/// # Image dimensions are too big.
	Overflow,

	/// # Image is too big.
	TooBig,

	#[cfg(feature = "bin")]
	/// # GTK failed.
	GtkInit,

	#[cfg(feature = "bin")]
	/// # No source image set.
	MissingSource,

	#[cfg(feature = "bin")]
	/// # No encoders enabled.
	NoEncoders,

	#[cfg(feature = "bin")]
	/// # Result was ont saved.
	NoSave,

	#[cfg(feature = "bin")]
	/// # I/O read error.
	Read,

	#[cfg(feature = "bin")]
	/// # I/O write error.
	Write,

	#[cfg(feature = "bin")]
	/// # Print Help (Not an Error).
	PrintHelp,

	#[cfg(feature = "bin")]
	/// # Print Version (Not an Error).
	PrintVersion,
}

impl Error for RefractError {}

impl AsRef<str> for RefractError {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl fmt::Display for RefractError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl RefractError {
	#[must_use]
	/// # As Str.
	///
	/// Return the error as an English string slice.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Color => "Unsupported color encoding format.",
			Self::Decode => "The image could not be decoded.",
			Self::Encode => "The image could not be encoded.",
			Self::Image => "Invalid image.",
			Self::ImageDecode(k) => match k {
				ImageKind::Avif => "Refract cannot decode AVIF images.",
				ImageKind::Jxl => "Refract cannot decode JPEG XL images.",
				ImageKind::Webp => "Refract cannot decode WebP images.",
				_ => "",
			},
			Self::ImageEncode(k) => match k {
				ImageKind::Jpeg => "Refract cannot encode JPEG files.",
				ImageKind::Png => "Refract cannot encode PNG files.",
				_ => "",
			},
			Self::NoBest(k) => match k {
				ImageKind::Avif => "No acceptable AVIF candidate was found.",
				ImageKind::Jxl => "No acceptable JPEG XL candidate was found.",
				ImageKind::Webp => "No acceptable WebP candidate was found.",
				_ => "",
			},
			Self::NothingDoing => "There is nothing else to do.",
			Self::Overflow => "The image dimensions are out of range.",
			Self::TooBig => "The encoded image was too big.",

			#[cfg(feature = "bin")]
			Self::GtkInit => "Failed to initialize GTK.",

			#[cfg(feature = "bin")]
			Self::MissingSource => "A source image must be set before a candidate image.",

			#[cfg(feature = "bin")]
			Self::NoEncoders => "At least one encoder must be enabled.",

			#[cfg(feature = "bin")]
			Self::NoSave => "The result was not saved.",

			#[cfg(feature = "bin")]
			Self::Read => "Unable to read the source file.",

			#[cfg(feature = "bin")]
			Self::Write => "Unable to save the file.",

			#[cfg(feature = "bin")]
			Self::PrintHelp => HELP,

			#[cfg(feature = "bin")]
			Self::PrintVersion => concat!("Refract v", env!("CARGO_PKG_VERSION")),
		}
	}
}
