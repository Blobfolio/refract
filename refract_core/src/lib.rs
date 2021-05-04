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



/// # Flag: AVIF Limited.
///
/// When enabled, color RGB sources will be encoded using the limited `YCbCr`
/// color space instead of the default full range, which typically leads to
/// smaller output.
///
/// This flag has no effect on greyscale images, which are always encoded using
/// the full range.
pub const FLAG_AVIF_LIMITED: u8 = 0b0001;
