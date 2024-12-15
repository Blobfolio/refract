/*!
# `Refract` - Library

This is the library powering [Refract](https://github.com/Blobfolio/refract), a guided CLI image encoding tool.
*/

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
	unsafe_code,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::module_name_repetitions, reason = "Repetition is preferred.")]
#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]

#[expect(unused_extern_crates, reason = "This is needed for JXL.")]
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
use std::num::NonZeroU8;



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

/// # 63 is Non-Zero.
pub(crate) const NZ_063: NonZeroU8 = NonZeroU8::new(63).unwrap();

/// # 100 is Non-Zero.
pub(crate) const NZ_100: NonZeroU8 = NonZeroU8::new(100).unwrap();

/// # 150 is Non-Zero.
pub(crate) const NZ_150: NonZeroU8 = NonZeroU8::new(150).unwrap();
