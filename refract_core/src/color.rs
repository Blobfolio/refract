/*!
# `Refract` - Color Helpers
*/

use imgref::Img;
use rgb::RGBA8;



#[derive(Debug, Clone, Copy)]
/// # Source Image Color.
///
/// We're always starting from `RGBA`, but we can help optimize instructions by
/// pre-calculating which channels are actually used.
pub enum ColorKind {
	/// # Greyscale.
	Grey,
	/// # Greyscale with Alpha.
	GreyAlpha,
	/// # RGB.
	Rgb,
	/// # RGB with Alpha.
	Rgba,
}

impl From<Img<&[RGBA8]>> for ColorKind {
	fn from(img: Img<&[RGBA8]>) -> Self {
		let alpha = img.pixels().any(|p| p.a != 255);
		let grey = img.pixels().all(|p| p.r == p.g && p.r == p.b);

		if alpha && grey { Self::GreyAlpha }
		else if alpha { Self::Rgba }
		else if grey { Self::Grey }
		else { Self::Rgb }
	}
}

/// # Getters.
impl ColorKind {
	#[must_use]
	/// # Color Channels.
	///
	/// Return the number of channels used by color, e.g. 3 for RGB.
	pub const fn color_channels(self) -> u32 {
		match self {
			Self::Grey | Self::GreyAlpha => 1,
			Self::Rgb | Self::Rgba => 3,
		}
	}

	#[must_use]
	/// # Extra Channels.
	///
	/// Return the number of extra channels, i.e. one for alpha.
	pub const fn extra_channels(self) -> u32 {
		match self {
			Self::GreyAlpha | Self::Rgba => 1,
			_ => 0,
		}
	}

	#[must_use]
	/// # Is Greyscale?
	///
	/// An image is greyscale if every pixel's individual R, G, and B values
	/// are equal.
	pub const fn is_greyscale(self) -> bool {
		matches!(self, Self::Grey | Self::GreyAlpha)
	}

	#[must_use]
	/// # Has Alpha?
	///
	/// If any pixel has alpha data associated with it, this is true.
	pub const fn has_alpha(self) -> bool {
		matches!(self, Self::GreyAlpha | Self::Rgba)
	}
}

/// # Helpers.
impl ColorKind {
	#[must_use]
	/// # Generate Buffer.
	pub fn to_buf(self, img: Img<&[RGBA8]>) -> Box<[u8]> {
		match self {
			// One channel.
			Self::Grey => img.pixels().map(|p| p.r).collect(),
			// Two channels.
			Self::GreyAlpha => img.pixels().fold(
				Vec::with_capacity(img.width() * img.height() * 2),
				|mut acc, p| {
					acc.extend_from_slice(&[p.r, p.a]);
					acc
				}
			),
			// Three channels.
			Self::Rgb => img.pixels().fold(
				Vec::with_capacity(img.width() * img.height() * 3),
				|mut acc, p| {
					acc.extend_from_slice(&[p.r, p.g, p.b]);
					acc
				}
			),
			// Four channels.
			Self::Rgba => img.pixels().fold(
				Vec::with_capacity(img.width() * img.height() * 4),
				|mut acc, p| {
					acc.extend_from_slice(&[p.r, p.g, p.b, p.a]);
					acc
				}
			),
		}.into_boxed_slice()
	}
}
