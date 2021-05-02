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
	Yuv,
}
