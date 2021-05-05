/*!
# `Refract` - Pixel Type
*/



#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// # Pixel Storage Kind.
///
/// The pixel buffers used by [`Image`](crate::Image) are either [`PixelKind::Full`] —
/// `[R, G, B, A, R, G, B, A…]` — or [`PixelKind::Compact`], meaning they only contain
/// the channels used by the image. For example, a greyscale image would just
/// be `[R,R,R…]`.
pub enum PixelKind {
	/// # RGBA (but just the used channels).
	Compact,
	/// # RGBA.
	Full,
	/// # YUV (YCbCr).
	///
	/// This is a special storage mode enabled by the [`FLAG_AVIF_LIMITED`] flag.
	/// The RGB buffer is converted from RGB into YUV (YCbCr) and stored
	/// contiguously with all the Ys first, then the Us, Vs, and As.
	YuvLimited,
	/// # YUV (GBR).
	///
	/// This is a special storage mode for AVIF encoding. The RGB buffer is
	/// converted to GBR/YUV and stored contiguously with all the Ys first,
	/// then the Us, Vs, and As.
	YuvFull,
}
