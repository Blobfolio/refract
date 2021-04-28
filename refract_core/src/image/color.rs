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
