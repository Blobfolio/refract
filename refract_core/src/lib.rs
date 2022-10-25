/*!
# `Refract` - Library

This is the library powering [Refract](https://github.com/Blobfolio/refract), a guided CLI image encoding tool.
*/

#![deny(unsafe_code)]

#![warn(
	clippy::filetype_is_file,
	clippy::integer_division,
	clippy::needless_borrow,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::suboptimal_flops,
	clippy::unneeded_field_pattern,
	macro_use_extern_crate,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![allow(
	clippy::module_name_repetitions,
	clippy::redundant_pub_crate,
)]

#[allow(unused_extern_crates)] // Needed for JXL.
extern crate link_cplusplus;

mod enc;
mod error;
mod input;
mod kind;
pub(crate) mod traits;



pub use enc::{
	iter::EncodeIter,
	output::Output,
	quality::{
		Quality,
		QualityValue,
	},
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
/// When enabled, only full-range `RGB` encoding will be attempted.
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

/// # (Internal) Encoder Flag: Valid Output.
///
/// This is used by [`Output`] to determine whether or not the buffer has been
/// validated.
pub(crate) const FLAG_VALID:        u8 = 0b0010_0000;

/// # (Internal) Encoder Flag: Tried Lossless.
///
/// This is used by [`EncodeIter`] to determine whether or not lossless
/// encoding needs to be completed during iteration.
pub(crate) const FLAG_DID_LOSSLESS: u8 = 0b0100_0000;
