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
	Shadow,
	Theme,
	theme::Palette,
	Vector,
	widget::{
		button,
		container,
		text_input,
	},
};
use std::sync::LazyLock;

/// # Named Colors.
pub(super) struct NiceColors;

/// # Helper: Color Constants.
macro_rules! nice_color {
	($($name:ident: $color:expr),+ $(,)?) => ($(
		#[doc = concat!("# ", stringify!($name))]
		pub(super) const $name: Color = $color;
	)+)
}

#[expect(dead_code, reason = "We might want these colors eventually.")]
impl NiceColors {
	nice_color!(
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
		TRANSPARENT: Color::TRANSPARENT,
	);
}

/// # Light Theme.
pub(super) static LIGHT_THEME: LazyLock<Theme> = LazyLock::new(|| Theme::custom("RefractLight".to_owned(), LIGHT_PALETTE));

/// # Light Color Palette.
pub(super) const LIGHT_PALETTE: Palette = Palette {
	background: NiceColors::WHITE,
	text:       NiceColors::BLACK,
	primary: 	NiceColors::BLUE,
	success: 	NiceColors::GREEN,
	// TODO: coming soon?
	// warning: NiceColors::ORANGE,
	danger:     NiceColors::RED,
};

/// # Dark Theme.
pub(super) static DARK_THEME: LazyLock<Theme> = LazyLock::new(|| Theme::custom("RefractDark".to_owned(), DARK_PALETTE));

/// # Dark Color Palette.
pub(super) const DARK_PALETTE: Palette = Palette {
	background: NiceColors::BLACK,
	text:       NiceColors::WHITE,
	primary: 	NiceColors::BLUE,
	success: 	NiceColors::GREEN,
	// TODO: coming soon?
	// warning: NiceColors::ORANGE,
	danger:     NiceColors::RED,
};

/// # Non-Shadow Shadow.
pub(super) const NO_SHADOW: Shadow = Shadow {
	color: NiceColors::TRANSPARENT,
	offset: Vector { x: 0.0, y: 0.0 },
	blur_radius: 0.0,
};



/// # Border Style.
const fn border_style(color: Color, width: f32, radius: f32) -> Border {
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
		text_color: NiceColors::WHITE,
		border: border_style(NiceColors::TRANSPARENT, 0.0, 8.0),
		shadow: NO_SHADOW,
	}
}

/// # Selectable Text Style.
///
/// This workaround styles a `TextInput` like a `Text` so that users can
/// select the contents (which for some reason isn't possible with regular
/// display widgets).
pub(super) const fn selectable_text_style(base: Color) -> text_input::Style {
	text_input::Style {
		background: Background::Color(NiceColors::TRANSPARENT),
		border: border_style(NiceColors::TRANSPARENT, 0.0, 0.0),
		icon: NiceColors::TRANSPARENT,
		placeholder: NiceColors::TRANSPARENT,
		value: base,
		selection: Color { a: 0.2, ..base },
	}
}

/// # Tooltip Style.
///
/// Light and dark variants for fun.
pub(super) const fn tooltip_style(light: bool) -> container::Style {
	let (bg, fg) =
		if light { (NiceColors::OFF_WHITE, NiceColors::BLACK) }
		else     { (NiceColors::OFF_BLACK, NiceColors::WHITE) };

	container::Style {
		text_color: Some(fg),
		background: Some(Background::Color(bg)),
		border: border_style(NiceColors::TEAL, 2.0, 0.0),
		shadow: NO_SHADOW,
	}
}
