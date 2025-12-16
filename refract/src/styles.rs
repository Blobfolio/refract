/*!
# Refract: Styles
*/

#![expect(clippy::unreadable_literal, reason = "Counter-productive for RGB hex.")]

use iced::{
	Background,
	Border,
	border::Radius,
	color,
	Color,
	Font,
	font,
	Padding,
	Pixels,
	Shadow,
	theme::Palette,
	Vector,
	widget::{
		button,
		container,
		scrollable,
	},
};



/// # Skin.
///
/// This struct holds style-related helpers and constants, taking some of the
/// ~~load~~ code off `App`.
pub(super) struct Skin;

/// # Helper: Color Constants.
macro_rules! nice_color {
	($($name:ident: $color:expr),+ $(,)?) => ($(
		#[doc = concat!("# ", stringify!($name))]
		pub(super) const $name: Color = $color;
	)+)
}

/// # Colors.
impl Skin {
	nice_color!(
		BABYFOOD:    color!(0xa49c00),
		BLACK:       color!(0x333333),
		BLUE:        color!(0x00abc0),
		GREEN:       color!(0x2ecc71),
		GREY:        color!(0xaaaaaa),
		OFF_BLACK:   color!(0x666666),
		OFF_WHITE:   color!(0xf0f0f0),
		ORANGE:      color!(0xe67e22),
		PINK:        color!(0xff3596),
		PURPLE:      color!(0x9b59b6),
		RED:         color!(0xe74c3c),
		TEAL:        color!(0x1abc9c),
		WHITE:       color!(0xffffff),
		YELLOW:      color!(0xfff200),
		YELLUCK:     color!(0xebe577),
		TRANSPARENT: Color::TRANSPARENT,
	);

	/// # Light Color Palette.
	pub(super) const LIGHT_PALETTE: Palette = Palette {
		background: Self::WHITE,
		text:       Self::BLACK,
		primary: 	Self::BLUE,
		success: 	Self::GREEN,
		warning:    Self::ORANGE,
		danger:     Self::RED,
	};

	/// # Dark Color Palette.
	pub(super) const DARK_PALETTE: Palette = Palette {
		background: Self::BLACK,
		text:       Self::WHITE,
		primary: 	Self::BLUE,
		success: 	Self::GREEN,
		warning:    Self::ORANGE,
		danger:     Self::RED,
	};

	/// # Foreground Color.
	pub(super) const fn fg(dark: bool) -> Color {
		if dark { Self::DARK_PALETTE.text }
		else { Self::LIGHT_PALETTE.text }
	}

	/// # Maybe Dim a Color.
	///
	/// Pass through the color if `cond`, otherwise dim it to 50% opacity.
	pub(super) const fn maybe_dim(color: Color, cond: bool) -> Color {
		if cond { color }
		else {
			Color { a: 0.5, ..color }
		}
	}
}

/// # Fonts and Text.
impl Skin {
	/// # Fira Mono: Regular.
	pub(super) const FONT: Font = Font {
		family:  font::Family::Name("Fira Mono"),
		weight:  font::Weight::Medium,
		stretch: font::Stretch::Normal,
		style:   font::Style::Normal,
	};

	/// # Tiny Font Size.
	pub(super) const TEXT_SM: Pixels = Pixels(12.0);

	/// # Normal Font Size.
	pub(super) const TEXT_MD: Pixels = Pixels(14.0);

	/// # Big Font Size.
	pub(super) const TEXT_LG: Pixels = Pixels(18.0);
}

/// # General Spacing.
impl Skin {
	/// # Quarter Gap.
	pub(super) const GAP25: f32 = 5.0;

	/// # Half Gap.
	pub(super) const GAP50: f32 = 10.0;

	/// # Three-Quarter Gap.
	pub(super) const GAP75: f32 = 15.0;

	/// # Normal/Full Gap.
	pub(super) const GAP: f32 = 20.0;
}

/// # Widgets.
impl Skin {
	/// # Button Padding.
	pub(super) const BTN_PADDING: Padding = Padding {
		top: 10.0,
		right: 20.0,
		bottom: 10.0,
		left: 20.0,
	};

	/// # Check Size.
	pub(super) const CHK_SIZE: Pixels = Pixels(12.0);

	/// # Image Scroller Rail.
	const IMG_SCROLL_RAIL: scrollable::Rail = scrollable::Rail {
		background: Some(Background::Color(Self::YELLUCK)),
		border: Self::border_style(Self::TRANSPARENT, 0.0, 0.0),
		scroller: scrollable::Scroller {
			background: Background::Color(Self::YELLOW),
			border: Self::border_style(Self::BABYFOOD, 2.0, 0.0),
		},
	};

	/// # Image Scroller.
	pub(super) const IMG_SCROLL: scrollable::Style = scrollable::Style {
		container: container::Style {
			text_color: None,
			background: None,
			border: Self::border_style(Self::TRANSPARENT, 0.0, 0.0),
			shadow: Self::NO_SHADOW,
			snap: true,
		},
		vertical_rail: Self::IMG_SCROLL_RAIL,
		horizontal_rail: Self::IMG_SCROLL_RAIL,
		gap: None,
		auto_scroll: scrollable::AutoScroll {
			background: Background::Color(Self::YELLOW),
			border: Self::border_style(Self::BABYFOOD, 2.0, 0.0),
			shadow: Self::NO_SHADOW,
			icon: Self::WHITE,
		}
	};

	/// # Non-Shadow Shadow.
	pub(super) const NO_SHADOW: Shadow = Shadow {
		color: Self::TRANSPARENT,
		offset: Vector { x: 0.0, y: 0.0 },
		blur_radius: 0.0,
	};

	/// # Tooltip Width.
	pub(super) const TOOLTIP_SIZE: Pixels = Pixels(300.0);

	/// # Border Style.
	pub(super) const fn border_style(color: Color, width: f32, radius: f32) -> Border {
		Border {
			color,
			width,
			radius: Radius {
				top_left: radius,
				top_right: radius,
				bottom_right: radius,
				bottom_left: radius,
			},
		}
	}

	/// # Button Style.
	///
	/// Produce a decent-looking button with the given base (background) color.
	/// Note that for our purposes, the text is always white.
	pub(super) const fn button_style(status: button::Status, base: Color) -> button::Style {
		button::Style {
			background: Some(Background::Color(match status {
				button::Status::Active => base,
				button::Status::Hovered | button::Status::Pressed => Color { a: 0.9, ..base },
				button::Status::Disabled => Color { a: 0.5, ..base },
			})),
			text_color: Self::WHITE,
			border: Self::border_style(Self::TRANSPARENT, 0.0, 8.0),
			shadow: Self::NO_SHADOW,
			snap: true,
		}
	}

	/// # Tooltip Style.
	///
	/// Light and dark variants for fun.
	pub(super) const fn tooltip_style(dark: bool) -> container::Style {
		let (bg, fg) =
			if dark { (Self::OFF_BLACK, Self::WHITE) }
			else    { (Self::OFF_WHITE, Self::BLACK) };

		container::Style {
			text_color: Some(fg),
			background: Some(Background::Color(bg)),
			border: Self::border_style(Self::TEAL, 2.0, 0.0),
			shadow: Self::NO_SHADOW,
			snap: true,
		}
	}
}
