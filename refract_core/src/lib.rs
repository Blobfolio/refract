/*!
# `Refract` - Library
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



mod enc;
mod error;
mod image;
mod source;


pub use enc::{
	candidate::Candidate,
	iter::EncodeIter,
	kind::OutputKind,
	output::Output,
};
pub use error::RefractError;
pub use image::{
	color::ColorKind,
	Image,
	pixel::PixelKind,
};
pub use source::{
	Source,
	SourceKind,
};



/// # Flag: Disable AVIF Limited Range
///
/// When set, limited ranges will never be tested.
pub const FLAG_NO_AVIF_LIMITED: u8     = 0b0000_0001;



/// # Internal Flag: AVIF RGB Mode.
///
/// This flag is present when encoding should follow full-range RGB mode.
pub(crate) const FLAG_AVIF_RGB: u8     = 0b0000_0010;

/// # Internal Flag: AVIF Round Two.
///
/// Round two is encoding with limited-range `YCbCr` mode.
pub(crate) const FLAG_AVIF_ROUND_2: u8 = 0b0000_1000;

/// # Internal Flag: AVIF Round Three.
///
/// Round three repeats the encoding of the "best" candidate with tiling
/// disabled.
pub(crate) const FLAG_AVIF_ROUND_3: u8 = 0b0001_0000;

/// # Internal Flag: Lossless.
///
/// This flag is present on output when it was encoded losslessly.
pub(crate) const FLAG_LOSSLESS:     u8 = 0b0010_0000;
