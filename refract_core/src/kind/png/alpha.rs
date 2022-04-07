/*!
# `Refract` - Alpha Operations.

The `ravif` crate's [dirtalpha](https://github.com/kornelski/cavif-rs/blob/main/ravif/src/dirtyalpha.rs)
module is super useful, but unfortunately we can't use it directly due to
dependency conflicts.

This is a recreation of that module (and its `loop9` dependency), better
tailored to this app's data design.
*/



#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
struct Nine([u8; 36]);

/// ## Getters.
impl Nine {
	#[inline]
	/// # The Center Pixel's Red.
	const fn red(&self) -> u8 { self.0[16] }

	#[inline]
	/// # The Center Pixel's Green.
	const fn green(&self) -> u8 { self.0[17] }

	#[inline]
	/// # The Center Pixel's Blue.
	const fn blue(&self) -> u8 { self.0[18] }

	#[inline]
	/// # The Center Pixel's Alpha.
	const fn alpha(&self) -> u8 { self.0[19] }

	#[inline]
	/// # Has Alpha?
	///
	/// This returns true if the center pixel's alpha channel is less than 255.
	const fn has_alpha(&self) -> bool { self.alpha() != 255 }

	/// # Has Invisible Pixels?
	///
	/// This returns true if any of the pixels in the set have an alpha value
	/// of zero.
	fn has_invisible(&self) -> bool { self.0.chunks_exact(4).any(|px| px[3] == 0) }

	#[inline]
	/// # Is Semi-Transparent?
	///
	/// This returns true if the center pixel's alpha channel is less than 255
	/// but greater than zero.
	const fn is_semi_transparent(&self) -> bool {
		0 < self.alpha() && self.alpha() < 255
	}
}



/// ## Calculations.
impl Nine {
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
	fn averaged(&self) -> Option<[u8; 4]> {
		let (r, g, b) = self.0.chunks_exact(4)
			.fold((0_u16, 0_u16, 0_u16), |mut acc, px| {
				acc.0 += u16::from(px[0]);
				acc.1 += u16::from(px[1]);
				acc.2 += u16::from(px[2]);
				acc
			});

		// This is a straight average of the entire block, which always
		// has nine members (even if some will be duplicates).
		self.normalize_avg(
			r.wrapping_div(9) as u8,
			g.wrapping_div(9) as u8,
			b.wrapping_div(9) as u8,
		)
	}

	/// # Make Averaged Pixel.
	///
	/// This puts the finishing touches on a pixel generated by [`Nine::averaged`]
	/// or [`Nine::weighted`], clamping values if necessary, and returning a
	/// formed RGBA slice if different than the current center.
	fn normalize_avg(&self, r: u8, g: u8, b: u8) -> Option<[u8; 4]> {
		let mut avg = [r, g, b, self.alpha()];

		// Unless this is invisible, we should clamp it.
		if avg[3] != 0 {
			avg[0] = clamp(avg[0], self.red(), self.alpha());
			avg[1] = clamp(avg[1], self.green(), self.alpha());
			avg[2] = clamp(avg[2], self.blue(), self.alpha());
		}

		if avg[..3] == self.0[16..19] { None }
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
	fn weighted(&self) -> Option<[u8; 4]> {
		let (r, g, b, weight) = self.0.chunks_exact(4)
			.fold((0_u32, 0_u32, 0_u32, 0_u32), |mut acc, px| {
				if px[3] > 0 {
					let weight = 256 - u32::from(px[3]);
					acc.0 += u32::from(px[0]) * weight;
					acc.1 += u32::from(px[1]) * weight;
					acc.2 += u32::from(px[2]) * weight;
					acc.3 += weight;
				}

				acc
			});

		// If there were visible neighbors, make the adjustment!
		if weight > 0 {
			self.normalize_avg(
				r.wrapping_div(weight) as u8,
				g.wrapping_div(weight) as u8,
				b.wrapping_div(weight) as u8,
			)
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
pub(super) fn clean_alpha(img: &mut [u8], width: usize, height: usize) {
	if let Some(avg) = neutral_pixel(img, width, height) {
		// Set all invisible pixels to said neutral color.
		img.chunks_exact_mut(4)
			.filter(|px| px[3] == 0)
			.for_each(|px| { px.copy_from_slice(&avg); });

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
fn blur_alpha(img: &mut [u8], width: usize, height: usize) {
	// First compute a weighted average.
	let mut diff: Vec<(usize, [u8; 4])> = Vec::new();
	let mut idx: usize = 0;
	the_nines(img, width, height, |n| {
		if n.has_alpha() {
			if let Some(avg) = n.weighted() {
				diff.push((idx, avg));
			}
		}
		idx += 4;
	});

	// Apply the changes.
	diff.drain(..).for_each(|(idx, px)| {
		img[idx..idx + 4].copy_from_slice(&px);
	});

	// Now compute a straight average.
	idx = 0;
	the_nines(img, width, height, |n| {
		if n.has_alpha() {
			if let Some(avg) = n.averaged() {
				diff.push((idx, avg));
			}
		}
		idx += 4;
	});

	// And apply it!
	for (idx, px) in diff {
		img[idx..idx + 4].copy_from_slice(&px);
	}
}

#[allow(clippy::cast_possible_truncation)] // Values will be in range.
#[inline]
/// # Clamp Pixel.
///
/// This prevents averaged/weighted pixel reassignments from drifting too far
/// from the original.
fn clamp(px_new: u8, px_old: u8, alpha: u8) -> u8 {
	// Leave some spare room for rounding.
	let alpha = u16::from(alpha);
	let rounded = (u16::from(px_old) * alpha).wrapping_div(255) * 255;
	let low = px_old.min((rounded + 16).wrapping_div(alpha) as u8);
	let high = px_old.max((rounded + 239).wrapping_div(alpha) as u8);

	px_new.max(low).min(high)
}

#[allow(clippy::cast_possible_truncation)] // Values will be in range.
#[allow(clippy::similar_names)] // Weight and Height are quite different!
/// # Neutral Pixel.
fn neutral_pixel(img: &[u8], width: usize, height: usize) -> Option<[u8; 4]> {
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
		Some([
			r.wrapping_div(t) as u8,
			g.wrapping_div(t) as u8,
			b.wrapping_div(t) as u8,
			0,
		])
	}
	else { None }
}

/// # Loop Pixels
///
/// Loop through the pixels of an image, producing a [`Nine`] for each,
/// containing all of the neighboring pixels (with the main one in the center).
fn the_nines<Cb>(img: &[u8], width: usize, height: usize, mut cb: Cb)
where Cb: FnMut(Nine) {
	let row_size = width << 2;

	// Make sure we have at least 3 pixels in either direction, and that the
	// buffer is the correct size.
	if width < 3 || height < 3 || img.len() != row_size * height { return; }

	let mut nine = Nine([0_u8; 36]);

	// Loop the rows.
	for y in 0..height {
		// Figure out the rows.
		let middle = y * row_size;
		let top = middle.saturating_sub(row_size);
		let bottom =
			if y + 1 < height { middle + row_size }
			else { middle };

		// Start each row with 0, 0, 1 columns. We know there's always going to
		// be a +1 because we refuse images with widths < 3.
		nine.0[..4].copy_from_slice(&img[top..top + 4]);
		nine.0[4..12].copy_from_slice(&img[top..top + 8]);

		nine.0[12..16].copy_from_slice(&img[middle..middle + 4]);
		nine.0[16..24].copy_from_slice(&img[middle..middle + 8]);

		nine.0[24..28].copy_from_slice(&img[bottom..bottom + 4]);
		nine.0[28..].copy_from_slice(&img[bottom..bottom + 8]);

		// Callback for X zero.
		cb(nine);

		// Loop the columns.
		for x in 1..width {
			// Shift the old middle and right positions down for each row.
			unsafe {
				let src = nine.0.as_ptr().add(4);
				let dst = nine.0.as_mut_ptr();

				std::ptr::copy(src, dst, 8);
				std::ptr::copy(src.add(12), dst.add(12), 8);
				std::ptr::copy(src.add(24), dst.add(24), 8);
			}

			// Copy in the new right positions, if any.
			if x + 1 < width {
				let right = (x + 1) << 2;
				nine.0[8..12].copy_from_slice(&img[top + right..top + right + 4]);
				nine.0[20..24].copy_from_slice(&img[middle + right..middle + right + 4]);
				nine.0[32..].copy_from_slice(&img[bottom + right..bottom + right + 4]);
			}

			// Callback for the rest!
			cb(nine);
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	/// # Min/Max Abstraction.
	///
	/// This is the min/max portion of [`Nine::clamp`] copy-and-pasted into a
	/// standalone method so we can verify the operations without having to
	/// look at the rest.
	fn premultiplied_minmax(px_old: u8, alpha: u8) -> (u8, u8) {
		let alpha = u16::from(alpha);
		let rounded = (u16::from(px_old) * alpha).wrapping_div(255) * 255;
		let low = px_old.min((rounded + 16).wrapping_div(alpha) as u8);
		let high = px_old.max((rounded + 239).wrapping_div(alpha) as u8);

		(low, high)
	}

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
		let mut img: Vec<u8> = Vec::new();
		for i in 0..16*4 { img.push(i); }

		// There should be 16 pixels total.
		assert_eq!(img.len(), 16 * 4);

		let mut idx: u8 = 0;
		the_nines(&img, 4, 4, |n| {
			match idx {
				// First row!
				0 => assert_eq!(
					n,
					Nine([
						0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7,
						0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7,
						16, 17, 18, 19, 16, 17, 18, 19, 20, 21, 22, 23,
					]),
				),
				1 => assert_eq!(
					n,
					Nine([
						0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
						0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
						16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
					]),
				),
				2 => assert_eq!(
					n,
					Nine([
						4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
						4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
						20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
					]),
				),
				3 => assert_eq!(
					n,
					Nine([
						8, 9, 10, 11, 12, 13, 14, 15, 12, 13, 14, 15,
						8, 9, 10, 11, 12, 13, 14, 15, 12, 13, 14, 15,
						24, 25, 26, 27, 28, 29, 30, 31, 28, 29, 30, 31,
					]),
				),
				// Row change!
				4 => assert_eq!(
					n,
					Nine([
						0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7,
						16, 17, 18, 19, 16, 17, 18, 19, 20, 21, 22, 23,
						32, 33, 34, 35, 32, 33, 34, 35, 36, 37, 38, 39,
					])
				),
				5 => assert_eq!(
					n,
					Nine([
						0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
						16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
						32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43,
					]),
				),
				6 => assert_eq!(
					n,
					Nine([
						4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
						20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
						36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
					]),
				),
				7 => assert_eq!(
					n,
					Nine([
						8, 9, 10, 11, 12, 13, 14, 15, 12, 13, 14, 15,
						24, 25, 26, 27, 28, 29, 30, 31, 28, 29, 30, 31,
						40, 41, 42, 43, 44, 45, 46, 47, 44, 45, 46, 47,
					]),
				),
				// Jump to the end.
				15 => assert_eq!(
					n,
					Nine([
						40, 41, 42, 43, 44, 45, 46, 47, 44, 45, 46, 47,
						56, 57, 58, 59, 60, 61, 62, 63, 60, 61, 62, 63,
						56, 57, 58, 59, 60, 61, 62, 63, 60, 61, 62, 63,
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

