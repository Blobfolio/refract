/*!
# `Refract` - Kinds
*/

#[cfg(feature = "avif")] pub(super) mod avif;
pub(super) mod color;
pub(super) mod image;
#[cfg(feature = "jpeg")] pub(super) mod jpeg;
#[cfg(feature = "jxl")]  pub(super) mod jxl;
#[cfg(feature = "png")]  pub(super) mod png;
#[cfg(feature = "webp")] pub(super) mod webp;
