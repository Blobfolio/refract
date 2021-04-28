/*!
# `Refract` - Alpha Operations.

This is a re-implementation of `ravif`'s [dirtalpha](https://github.com/kornelski/cavif-rs/blob/main/ravif/src/dirtyalpha.rs)
module, which cannot be imported as a library due to dependency conflicts. :(
*/

use imgref::{
	Img,
	ImgRef,
};
use rgb::{
	ComponentMap,
	RGB,
	RGBA8,
};



#[allow(clippy::cast_possible_truncation)]
/// # Clear Alpha Channels.
///
/// This removes fully transparent pixels from RGB components to make them
/// cheaper to encode, particularly with AV1.
///
/// This is applied to each imported image source.
pub(super) fn clear_alpha(mut img: Img<Vec<RGBA8>>) -> Img<Vec<RGBA8>> {
	// Get the dominant visible transparent color (excluding opaque pixels).
	let mut sum = RGB::new(0, 0, 0);
	let mut weights = 0;

	// Only consider colors around transparent images. Solid semi-transparent
	// areas don't need to contribute.
	loop9::loop9_img(img.as_ref(), |_, _, top, mid, bot| {
		if mid.curr.a == 255 || mid.curr.a == 0 {
			return;
		}

		if chain(&top, &mid, &bot).any(|px| px.a == 0) {
			let (w, px) = weighed_pixel(mid.curr);
			weights += u64::from(w);
			sum += px.map(u64::from);
		}
	});

	// Opaque image.
	if weights == 0 { return img; }

	let neutral_alpha = RGBA8::new(
		num_integer::div_floor(sum.r, weights) as u8,
		num_integer::div_floor(sum.g, weights) as u8,
		num_integer::div_floor(sum.b, weights) as u8,
		0
	);

	img.pixels_mut()
		.filter(|px| px.a == 0)
		.for_each(|px| *px = neutral_alpha);

	let img2 = bleed_opaque_color(img.as_ref());
	drop(img);
	blur_transparent_pixels(img2.as_ref())
}

#[allow(clippy::cast_possible_truncation)]
/// # Copy Color.
///
/// Copy the color from opaque pixels to transparent pixels. This way when
/// edges get crushed by compression, the distortion will be away from the
/// visible edge.
fn bleed_opaque_color(img: ImgRef<RGBA8>) -> Img<Vec<RGBA8>> {
	let mut out = Vec::with_capacity(img.width() * img.height());
	loop9::loop9_img(img, |_, _, top, mid, bot| {
		out.push(
			if mid.curr.a == 255 { mid.curr }
			else {
				let (weights, sum) = chain(&top, &mid, &bot)
					.map(|c| weighed_pixel(*c))
					.fold((0_u32, RGB::new(0,0,0)), |mut sum, item| {
						sum.0 += u32::from(item.0);
						sum.1 += item.1;
						sum
					});

				if weights == 0 { mid.curr }
				else {
					let mut avg = sum.map(|c| num_integer::div_floor(c, weights) as u8);
					if mid.curr.a == 0 {
						avg.alpha(0)
					}
					else {
						// Also change non-transparent colors, but only within
						// the range where rounding caused by premultiplied
						// alpha would lead to the same color.
						avg.r = clamp(avg.r, premultiplied_minmax(mid.curr.r, mid.curr.a));
						avg.g = clamp(avg.g, premultiplied_minmax(mid.curr.g, mid.curr.a));
						avg.b = clamp(avg.b, premultiplied_minmax(mid.curr.b, mid.curr.a));
						avg.alpha(mid.curr.a)
					}
				}
			}
		);
	});

	Img::new(out, img.width(), img.height())
}

#[allow(clippy::cast_possible_truncation)]
/// # Remove Sharp Edges.
///
/// Remove any sharp edges created by the cleared alpha.
fn blur_transparent_pixels(img: ImgRef<RGBA8>) -> Img<Vec<RGBA8>> {
	let mut out = Vec::with_capacity(img.width() * img.height());
	loop9::loop9_img(img, |_, _, top, mid, bot| {
		out.push(
			if mid.curr.a == 255 { mid.curr }
			else {
				let sum: RGB<u16> = chain(&top, &mid, &bot)
					.map(|px| px.rgb().map(u16::from))
					.sum();

				let mut avg = sum.map(|c| num_integer::div_floor(c, 9) as u8);

				if mid.curr.a == 0 { avg.alpha(0) }
				else {
					// Also change transparent colors, but only within the
					// range where rounding caused by premultiplied alpha
					// would lead to the same color.
					avg.r = clamp(avg.r, premultiplied_minmax(mid.curr.r, mid.curr.a));
					avg.g = clamp(avg.g, premultiplied_minmax(mid.curr.g, mid.curr.a));
					avg.b = clamp(avg.b, premultiplied_minmax(mid.curr.b, mid.curr.a));
					avg.alpha(mid.curr.a)
				}
			}
		);
	});

	Img::new(out, img.width(), img.height())
}

#[allow(clippy::inline_always)]
#[inline(always)]
/// # Chain Helper.
fn chain<'a, T>(top: &'a loop9::Triple<T>, mid: &'a loop9::Triple<T>, bot: &'a loop9::Triple<T>) -> impl Iterator<Item = &'a T> + 'a {
	top.iter().chain(mid.iter()).chain(bot.iter())
}

#[inline]
/// # Clamp Helper.
fn clamp(px: u8, (min, max): (u8, u8)) -> u8 {
	px.max(min).min(max)
}

#[allow(clippy::cast_possible_truncation)]
#[inline]
/// # Premultiply Range.
///
/// Come up with a safe range to change pixel color given its alpha. Colors
/// with high transparency tolerate more variation.
fn premultiplied_minmax(px: u8, alpha: u8) -> (u8, u8) {
	let alpha = u16::from(alpha);
	let rounded = u16::from(px) * num_integer::div_floor(alpha, 255) * 255;

	// Leave some spare room for rounding.
	let low = num_integer::div_floor(rounded + 16, alpha) as u8;
	let hi = num_integer::div_floor(rounded + 239, alpha) as u8;

	(low.min(px), hi.max(px))
}

#[inline]
/// # Pixel Weight.
const fn weighed_pixel(px: RGBA8) -> (u16, RGB<u32>) {
	if px.a == 0 {
		return (0, RGB::new(0,0,0))
	}

	let weight = 256 - px.a as u16;

	(
		weight,
		RGB::new(
			px.r as u32 * weight as u32,
			px.g as u32 * weight as u32,
			px.b as u32 * weight as u32
		)
	)
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn preminmax() {
		assert_eq!((100, 100), premultiplied_minmax(100, 255));
		assert_eq!((78, 100), premultiplied_minmax(100, 10));
		assert_eq!(100 * 10 / 255, 78 * 10 / 255);
		assert_eq!(100 * 10 / 255, 100 * 10 / 255);
		assert_eq!((8, 119), premultiplied_minmax(100, 2));
		assert_eq!((16, 239), premultiplied_minmax(100, 1));
		assert_eq!((15, 255), premultiplied_minmax(255, 1));
	}
}
