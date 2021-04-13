/*!
# `Refract`: The Hard Bits
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



mod candidate;
mod encoder;
mod error;
mod image;
mod kind;
mod quality;
mod refraction;

pub use candidate::Candidate;
pub use encoder::Encoder;
pub use error::RefractError;
pub use image::Image;
pub use kind::ImageKind;
pub use quality::{
	MAX_QUALITY,
	MIN_QUALITY,
	Quality,
};
pub use refraction::Refraction;
