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
	/// # Only Used Channels.
	Compact,
	/// # RGBA.
	Full,
	/// # YUV.
	///
	/// This is a special mode used by AVIF when the [`FLAG_AVIF_LIMITED`] flag
	/// is set. This indicates the buffer has been converted from RGB into YUV,
	/// stored contiguously with all the Ys first, then the Us, Vs, and finally
	/// As.
	Yuv,
}
