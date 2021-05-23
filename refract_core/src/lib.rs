/*!
# `Refract` - Library

This is the library powering [Refract](https://github.com/Blobfolio/refract), a guided CLI image encoding tool.
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
mod input;
mod kind;
pub(crate) mod traits;



pub use enc::{
	iter::EncodeIter,
	output::Output,
	quality::Quality,
	range::QualityRange,
};
pub use error::RefractError;
pub use input::Input;
pub use kind::{
	color::ColorKind,
	image::ImageKind,
};
pub(crate) use kind::{
	avif::ImageAvif,
	jpeg::ImageJpeg,
	jxl::ImageJxl,
	png::ImagePng,
	webp::ImageWebp,
};



/// # Encoder Flag: Disable Lossy Encoding.
///
/// When enabled, only lossless compression will be attempted.
pub const FLAG_NO_LOSSY: u8            = 0b0000_0001;

/// # Encoder Flag: Disable Lossless Encoding.
///
/// When enabled, only lossy compression will be attempted.
pub const FLAG_NO_LOSSLESS: u8         = 0b0000_0010;

/// # Encoder Flag: Disable `YCbCr` Encoding.
///
/// By default, `AVIF` encoding runs through both full-range `RGB` and limited-
/// range `YCbCr` modes. The latter often, but not always, saves a few
/// additional bytes, but it does also increase the encoding time quite a bit.
///
/// When enabled, only full-range `RGB` encoding will be attemped.
pub const FLAG_NO_AVIF_YCBCR: u8       = 0b0000_0100;

/// # (Internal) Encoder Flag: Public Flags Mask.
///
/// These are flags that can be set externally.
pub(crate) const PUBLIC_FLAGS: u8      = 0b0000_0111;

/// # (Internal) Encoder Flag: `AVIF` RGB.
///
/// When set, `AVIF` encoding will use `RGB` colors. When not set, it will use
/// `YCbCr`.
pub(crate) const FLAG_AVIF_RGB: u8     = 0b0000_1000;

/// # (Internal) Encoder Flag: `AVIF` Round Two.
///
/// The second `AVIF` encoding stage retries all quality ranges using `YCbCr`
/// color compression.
pub(crate) const FLAG_AVIF_ROUND_2: u8 = 0b0001_0000;

/// # (Internal) Encoder Flag: `AVIF` Round Three.
///
/// The final `AVIF` encoding stage reattempts conversion of the previous best
/// with tiling optimizations disabled. This is slow, hence only done once, but
/// will often shave off a few additional bytes.
pub(crate) const FLAG_AVIF_ROUND_3: u8 = 0b0010_0000;

/// # (Internal) Encoder Flag: Valid Output.
///
/// This is used by [`Output`] to determine whether or not the buffer has been
/// validated.
pub(crate) const FLAG_VALID:        u8 = 0b0100_0000;

/// # (Internal) Encoder Flag: Tried Lossless.
///
/// This is used by [`EncodeIter`] to determine whether or not lossless
/// encoding needs to be completed during iteration.
pub(crate) const FLAG_DID_LOSSLESS: u8 = 0b1000_0000;
