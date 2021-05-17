/*!
# `Refract` - Alpha Operations.

The `ravif` crate's [dirtalpha](https://github.com/kornelski/cavif-rs/blob/main/ravif/src/dirtyalpha.rs)
module is super useful, but unfortunately we can't use it directly due to
dependency conflicts.

This is a recreation of that module (and its `loop9` dependency), better
tailored to this app's data design.
*/

use rgb::RGBA8;



#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
/// # A Square of Nine Pixels.
///
/// This represents a pixel — located in the middle — and all eight of its
/// immediate neighbors.
///
/// At the edges of an image, "unavailable" neighbors are represented by
/// duplicating the corresponding last one. For example, at coordinate 0,0,
/// the top and middle rows will be identical, as will the left and center
/// columns within each row. At coordinate width,height, the middle and bottom
/// rows will match, as will the center and right columns within each row.
struct Nine([RGBA8; 9]);

/// ## Getters.
impl Nine {
	/// # The Center Pixel's Red.
	const fn red(&self) -> u8 { self.0[4].r }

	/// # The Center Pixel's Green.
	const fn green(&self) -> u8 { self.0[4].g }

	/// # The Center Pixel's Blue.
	const fn blue(&self) -> u8 { self.0[4].b }

	/// # The Center Pixel's Alpha.
	const fn alpha(&self) -> u8 { self.0[4].a }

	/// # Has Alpha?
	///
	/// This returns true if the center pixel's alpha channel is less than 255.
	const fn has_alpha(&self) -> bool { self.0[4].a != 255 }

	/// # Has Invisible Pixels?
	///
	/// This returns true if any of the pixels in the set have an alpha value
	/// of zero.
	fn has_invisible(&self) -> bool { self.0.iter().any(|px| px.a == 0) }

	/// # Is Semi-Transparent?
	///
	/// This returns true if the center pixel's alpha channel is less than 255
	/// but greater than zero.
	const fn is_semi_transparent(&self) -> bool {
		0 < self.0[4].a && self.0[4].a < 255
	}

	#[allow(clippy::cast_possible_truncation)] // Values will be in range.
	/// # Average.
	///
	/// This is a straight average of all of the pixels in a given set.
	///
	/// For visible center pixels, the result is clamped to prevent too much
	/// drift.
	///
	/// If the result turns out to be identical to the original value, `None`
	/// is returned.
	fn averaged(&self) -> Option<RGBA8> {
		let (r, g, b) = self.0.iter()
			.fold((0_u16, 0_u16, 0_u16), |mut acc, px| {
				acc.0 += u16::from(px.r);
				acc.1 += u16::from(px.g);
				acc.2 += u16::from(px.b);
				acc
			});

		// This is a straight average of the entire block, which always
		// has nine members (even if some will be duplicates).
		let mut avg = RGBA8::new(
			num_integer::div_floor(r, 9) as u8,
			num_integer::div_floor(g, 9) as u8,
			num_integer::div_floor(b, 9) as u8,
			self.alpha(),
		);

		// Clamp values to keep them from straying too far afield.
		if self.alpha() != 0 {
			avg.r = clamp(avg.r, self.red(), self.alpha());
			avg.b = clamp(avg.b, self.green(), self.alpha());
			avg.g = clamp(avg.g, self.blue(), self.alpha());
		}

		// Return if different!
		if self.0[4] == avg { None }
		else { Some(avg) }
	}

	#[allow(clippy::cast_possible_truncation)] // Values will be in range.
	/// # Weighted Average.
	///
	/// This calculates a weighted average of pixels (with alpha data) in the
	/// set. The more transparent a given pixel is, the more wiggle room we
	/// have in optimizing its color.
	///
	/// For visible center pixels, the result is clamped to prevent too much
	/// drift.
	///
	/// If no weighting is possible, or if the result winds up identical to the
	/// original, `None` is returned.
	fn weighted(&self) -> Option<RGBA8> {
		let (r, g, b, weight) = self.0.iter()
			.fold((0_u32, 0_u32, 0_u32, 0_u32), |mut acc, px| {
				if px.a > 0 {
					let weight = 256 - u32::from(px.a);
					acc.0 += u32::from(px.r) * weight;
					acc.1 += u32::from(px.g) * weight;
					acc.2 += u32::from(px.b) * weight;
					acc.3 += weight;
				}

				acc
			});

		// If there were visible neighbors, make the adjustment!
		if weight > 0 {
			let mut avg = RGBA8::new(
				num_integer::div_floor(r, weight) as u8,
				num_integer::div_floor(g, weight) as u8,
				num_integer::div_floor(b, weight) as u8,
				self.alpha(),
			);

			// Clamp values to keep them from straying too far afield.
			if self.alpha() != 0 {
				avg.r = clamp(avg.r, self.red(), self.alpha());
				avg.g = clamp(avg.g, self.green(), self.alpha());
				avg.b = clamp(avg.b, self.blue(), self.alpha());
			}

			// Return if different!
			if self.0[4] == avg { None }
			else { Some(avg) }
		}
		else { None }
	}
}



/// # Clean Up the Alpha!
///
/// For images with alpha channel data, three rounds of optimizations are
/// performed to improve later encoder efficiency and output compression:
///
/// * Fully transparent pixels are assigned a weighted, neutral color.
/// * Pixels with any degree of transparency appearing next to visible pixels have their colors shifted to a weighted average of said neighbors.
/// * Those same pixels are then averaged again to smooth out the edges.
///
/// Images without any alpha channel data are passed through unchanged.
pub(super) fn clean_alpha(img: &mut Vec<RGBA8>, width: usize, height: usize) {
	if let Some(avg) = neutral_pixel(img, width, height) {
		// Set all invisible pixels to said neutral color.
		img.iter_mut()
			.filter(|px| px.a == 0)
			.for_each(|px| { *px = avg; });

		// Visible pixels with transparency require more regional sensitivity to
		// avoid undesirable distortion. This is done with two rounds of averaging.
		blur_alpha(img, width, height);
	}
}

/// # Blur Alpha.
///
/// This optimization pass adjusts the colors of transparent pixels (visible or
/// otherwise) appearing next to visible pixels.
///
/// The less visible a pixel is, the more we can shift it.
fn blur_alpha(img: &mut Vec<RGBA8>, width: usize, height: usize) {
	// First compute a weighted average.
	let mut diff: Vec<(usize, RGBA8)> = Vec::new();
	let mut idx: usize = 0;
	the_nines(img, width, height, |n| {
		if n.has_alpha() {
			if let Some(avg) = n.weighted() {
				diff.push((idx, avg));
			}
		}
		idx += 1;
	});

	// Apply the changes.
	diff.drain(..).for_each(|(idx, px)| { img[idx] = px; });

	// Now compute a straight average.
	idx = 0;
	the_nines(img, width, height, |n| {
		if n.has_alpha() {
			if let Some(avg) = n.averaged() {
				diff.push((idx, avg));
			}
		}
		idx += 1;
	});

	// And apply it!
	diff.into_iter().for_each(|(idx, px)| { img[idx] = px; });
}

#[inline]
/// # Clamp Helper.
///
/// These lines get a little long...
fn clamp(px: u8, old: u8, a: u8) -> u8 {
	let (min, max) = premultiplied_minmax(old, a);
	px.max(min).min(max)
}

#[allow(clippy::cast_possible_truncation)] // Values will be in range.
#[allow(clippy::similar_names)] // Weight and Height are quite different!
/// # Neutral Pixel.
fn neutral_pixel(img: &[RGBA8], width: usize, height: usize) -> Option<RGBA8> {
	// First up, let's look for semi-transparent pixels appearing next to fully
	// transparent pixels, and average them up to create a suitable "default"
	// to apply to invisible pixels image-wide.
	let mut r: u64 = 0;
	let mut g: u64 = 0;
	let mut b: u64 = 0;
	let mut t: u64 = 0;
	the_nines(img, width, height, |n| {
		if n.is_semi_transparent() && n.has_invisible() {
			let weight = 256 - u64::from(n.alpha());
			r += u64::from(n.red()) * weight;
			g += u64::from(n.green()) * weight;
			b += u64::from(n.blue()) * weight;
			t += weight;
		}
	});

	// We only need to continue if we found the pixels we were looking for.
	if 0 < t {
		// Finish the average calculation to give us the neutral color.
		Some(RGBA8::new(
			num_integer::div_floor(r, t) as u8,
			num_integer::div_floor(g, t) as u8,
			num_integer::div_floor(b, t) as u8,
			0,
		))
	}
	else { None }
}

#[allow(clippy::cast_possible_truncation)] // Values are in range.
/// # Premultiply Range.
///
/// Come up with a safe range to change pixel color given its alpha. Colors
/// with high transparency tolerate more variation.
fn premultiplied_minmax(px: u8, alpha: u8) -> (u8, u8) {
	let alpha = u16::from(alpha);
	let rounded = num_integer::div_floor(u16::from(px) * alpha, 255) * 255;

	// Leave some spare room for rounding.
	let low = num_integer::div_floor(rounded + 16, alpha) as u8;
	let hi = num_integer::div_floor(rounded + 239, alpha) as u8;

	(low.min(px), hi.max(px))
}

/// # Loop Pixels
///
/// Loop through the pixels of an image, producing a [`Nine`] for each,
/// containing all of the neighboring pixels (with the main one in the center).
fn the_nines<Cb>(img: &[RGBA8], width: usize, height: usize, mut cb: Cb)
where Cb: FnMut(Nine) {
	// Make sure we have at least 3 pixels in either direction, and that the
	// buffer is the correct size.
	if width < 3 || height < 3 || img.len() != width * height { return; }

	let mut nine = Nine::default();

	// Loop the rows.
	for y in 0..height {
		// Figure out the rows.
		let middle = y * width;
		let top = middle.saturating_sub(width);
		let bottom =
			if y + 1 < height { middle + width }
			else { middle };

		// Start each row with 0, 0, 1 columns. We know there's always going to
		// be a +1 because we refuse images with widths < 3.
		nine.0[0] = img[top];
		nine.0[1] = img[top];
		nine.0[2] = img[top + 1];
		nine.0[3] = img[middle];
		nine.0[4] = img[middle];
		nine.0[5] = img[middle + 1];
		nine.0[6] = img[bottom];
		nine.0[7] = img[bottom];
		nine.0[8] = img[bottom + 1];

		// Loop the columns.
		for x in 0..width {
			// X=0 is set by the outer loop; everything else requires shifting.
			if x > 0 {
				// Shift the old middle and right positions down for each row.
				nine.0[..3].rotate_left(1);
				nine.0[3..6].rotate_left(1);
				nine.0[6..].rotate_left(1);

				// Copy in the new right positions, if any.
				let right =
					if x + 1 < width { x + 1 }
					else { x };

				nine.0[2] = img[top + right];
				nine.0[5] = img[middle + right];
				nine.0[8] = img[bottom + right];
			}

			// Run the callback!
			cb(nine);
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_preminmax() {
		assert_eq!((100, 100), premultiplied_minmax(100, 255));
		assert_eq!((78, 100), premultiplied_minmax(100, 10));
		assert_eq!(100 * 10 / 255, 78 * 10 / 255);
		assert_eq!(100 * 10 / 255, 100 * 10 / 255);
		assert_eq!((8, 119), premultiplied_minmax(100, 2));
		assert_eq!((16, 239), premultiplied_minmax(100, 1));
		assert_eq!((15, 255), premultiplied_minmax(255, 1));
	}

	#[test]
	fn t_nine() {
		let mut raw: Vec<u8> = Vec::new();
		for i in 0..16*4 { raw.push(i); }

		// Make a buffer with RGBA8 pixels.
		let img: Vec<RGBA8> = raw.chunks_exact(4)
			.map(|px| RGBA8::new(px[0], px[1], px[2], px[3]))
			.collect();

		// There should be 16 pixels total.
		assert_eq!(img.len(), 16);

		let mut idx: u8 = 0;
		the_nines(&img, 4, 4, |n| {
			match idx {
				// First row!
				0 => assert_eq!(
					n,
					Nine([
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(16, 17, 18, 19),
						RGBA8::new(16, 17, 18, 19),
						RGBA8::new(20, 21, 22, 23)
					]),
				),
				1 => assert_eq!(
					n,
					Nine([
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(16, 17, 18, 19),
						RGBA8::new(20, 21, 22, 23),
						RGBA8::new(24, 25, 26, 27)
					]),
				),
				2 => assert_eq!(
					n,
					Nine([
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(20, 21, 22, 23),
						RGBA8::new(24, 25, 26, 27),
						RGBA8::new(28, 29, 30, 31),
					]),
				),
				3 => assert_eq!(
					n,
					Nine([
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(24, 25, 26, 27),
						RGBA8::new(28, 29, 30, 31),
						RGBA8::new(28, 29, 30, 31),
					]),
				),
				// Row change!
				4 => assert_eq!(
					n,
					Nine([
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(16, 17, 18, 19),
						RGBA8::new(16, 17, 18, 19),
						RGBA8::new(20, 21, 22, 23),
						RGBA8::new(32, 33, 34, 35),
						RGBA8::new(32, 33, 34, 35),
						RGBA8::new(36, 37, 38, 39),
					])
				),
				5 => assert_eq!(
					n,
					Nine([
						RGBA8::new(0, 1, 2, 3),
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(16, 17, 18, 19),
						RGBA8::new(20, 21, 22, 23),
						RGBA8::new(24, 25, 26, 27),
						RGBA8::new(32, 33, 34, 35),
						RGBA8::new(36, 37, 38, 39),
						RGBA8::new(40, 41, 42, 43),
					]),
				),
				6 => assert_eq!(
					n,
					Nine([
						RGBA8::new(4, 5, 6, 7),
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(20, 21, 22, 23),
						RGBA8::new(24, 25, 26, 27),
						RGBA8::new(28, 29, 30, 31),
						RGBA8::new(36, 37, 38, 39),
						RGBA8::new(40, 41, 42, 43),
						RGBA8::new(44, 45, 46, 47),
					]),
				),
				7 => assert_eq!(
					n,
					Nine([
						RGBA8::new(8, 9, 10, 11),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(12, 13, 14, 15),
						RGBA8::new(24, 25, 26, 27),
						RGBA8::new(28, 29, 30, 31),
						RGBA8::new(28, 29, 30, 31),
						RGBA8::new(40, 41, 42, 43),
						RGBA8::new(44, 45, 46, 47),
						RGBA8::new(44, 45, 46, 47),
					]),
				),
				// Jump to the end.
				15 => assert_eq!(
					n,
					Nine([
						RGBA8::new(40, 41, 42, 43),
						RGBA8::new(44, 45, 46, 47),
						RGBA8::new(44, 45, 46, 47),
						RGBA8::new(56, 57, 58, 59),
						RGBA8::new(60, 61, 62, 63),
						RGBA8::new(60, 61, 62, 63),
						RGBA8::new(56, 57, 58, 59),
						RGBA8::new(60, 61, 62, 63),
						RGBA8::new(60, 61, 62, 63),
					])
				),
				_ => {}
			}

			idx += 1;
		});

		// Make sure we hit everything.
		assert_eq!(idx, 16);
	}
}

