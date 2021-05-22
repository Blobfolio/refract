/*!
# `Refract` - Color Kind
*/



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Source Image Color.
///
/// This is a list of color types, or, more specifically, which color
/// channels are used by a given image.
///
/// Alpha — [`ColorKind::GreyAlpha`] and [`ColorKind::Rgba`] — require at least
/// one alpha value being less than `255`.
///
/// Greyscale — [`ColorKind::Grey`] and [`ColorKind::GreyAlpha`] — require that
/// every RGB set have equal R, G, and B values.
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

/// # Getters.
impl ColorKind {
	#[inline]
	#[must_use]
	/// # Total Channels.
	///
	/// Return the number of channels.
	pub const fn channels(self) -> u32 {
		match self {
			Self::Grey => 1,
			Self::GreyAlpha => 2,
			Self::Rgb => 3,
			Self::Rgba => 4,
		}
	}

	#[inline]
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

	#[inline]
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

	#[inline]
	#[must_use]
	/// # Is Color?
	///
	/// An image is color if it contains at least one pixel with an R, G,
	/// and/or B that are not all the same.
	pub const fn is_color(self) -> bool {
		matches!(self, Self::Rgb | Self::Rgba)
	}

	#[inline]
	#[must_use]
	/// # Is Greyscale?
	///
	/// An image is greyscale if every pixel's individual R, G, and B values
	/// are equal.
	pub const fn is_greyscale(self) -> bool {
		matches!(self, Self::Grey | Self::GreyAlpha)
	}

	#[inline]
	#[must_use]
	/// # Has Alpha?
	///
	/// If any pixel has alpha data associated with it, this is true.
	pub const fn has_alpha(self) -> bool {
		matches!(self, Self::GreyAlpha | Self::Rgba)
	}
}

/// # Setters.
impl ColorKind {
	#[must_use]
	/// # From RGBA.
	///
	/// Find out whether the 4-byte pixel slice is using any color or alpha
	/// channels.
	pub fn from_rgba(src: &[u8]) -> Self {
		let mut color: bool = false;
		let mut alpha: bool = false;
		for px in src.chunks_exact(4) {
			if ! color && (px[0] != px[1] || px[0] != px[2]) {
				color = true;
				if alpha { return Self::Rgba; }
			}
			if ! alpha && px[3] != 255 {
				alpha = true;
				if color { return Self::Rgba; }
			}
		}

		// RGBA will have already been returned if applicable. If we're here,
		// it's one of the other three.
		if color { Self::Rgb }
		else if alpha { Self::GreyAlpha }
		else { Self::Grey }
	}
}
