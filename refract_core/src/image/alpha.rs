/*!
# `Refract` - Alpha Operations.

The `ravif` crate's [dirtalpha](https://github.com/kornelski/cavif-rs/blob/main/ravif/src/dirtyalpha.rs)
module is super useful, but unfortunately we can't use it directly due to
dependency conflicts.

This is a recreation of that module (and `loop9`), better tailored to this
app's data design.
*/



/// # Flag: Has Alpha (i.e. 0-254).
const FLAG_ALPHA: u8   = 0b0001;

/// # Flag: Is Visible (i.e. a != 0).
const FLAG_VISIBLE: u8 = 0b0010;

/// # Flag: Semi-Transparent.
const FLAG_SEMI_TRANSPARENT: u8 = 0b0011;



/// # Squares of Nine Pixels.
///
/// This is an iterator version of `loop9` that works on a 4-byte RGBA slice
/// rather than an `ImgVec`. It loops through the pixels of an image — `width *
/// height` iterations — returning a block of [`Nine`] at each run.
struct Nines<'a> {
	buf: &'a [u8],
	width: usize,
	height: usize,
	x: usize,
	y: usize,
}

impl<'a> Nines<'a> {
	#[inline]
	/// # New.
	///
	/// Start a new iterator from a source buffer.
	const fn new(src: &'a [u8], width: usize, height: usize) -> Self {
		Self {
			buf: src,
			width,
			height,
			x: 0,
			y: 0,
		}
	}
}

impl<'a> Iterator for Nines<'a> {
	type Item = Nine;

	fn next(&mut self) -> Option<Self::Item> {
		// It's over!
		if self.y == self.height {
			return None;
		}

		// Figure out the rows.
		let row_size = self.width * 4;
		let middle = self.y * row_size;
		let top = middle.saturating_sub(row_size);
		let bottom =
			if self.y + 1 < self.height { (self.y + 1) * row_size }
			else { middle };

		// Now the columns.
		let center = self.x * 4;
		let left = center.saturating_sub(4);
		let right =
			if center + 4 < row_size { center + 4 }
			else { center };

		let mut set = Nine::default();

		// We can set each slot now. If left/center/right hold three distinct
		// pixels, we can save some time by copying larger slices.
		if right - left == 8 {
			set.0[..12].copy_from_slice(&self.buf[top + left..top + right + 4]);
			set.0[12..24].copy_from_slice(&self.buf[middle + left..middle + right + 4]);
			set.0[24..].copy_from_slice(&self.buf[bottom + left..bottom + right + 4]);
		}
		// Otherwise let's just handle each chunk separately. We could optimize
		// for the left and right edges, but that won't hit often enough to
		// justify the verbosity. Haha.
		else {
			set.0[..4].copy_from_slice(&self.buf[top + left..top + left + 4]);
			set.0[4..8].copy_from_slice(&self.buf[top + center..top + center + 4]);
			set.0[8..12].copy_from_slice(&self.buf[top + right..top + right + 4]);
			set.0[12..16].copy_from_slice(&self.buf[middle + left..middle + left + 4]);
			set.0[16..20].copy_from_slice(&self.buf[middle + center..middle + center + 4]);
			set.0[20..24].copy_from_slice(&self.buf[middle + right..middle + right + 4]);
			set.0[24..28].copy_from_slice(&self.buf[bottom + left..bottom + left + 4]);
			set.0[28..32].copy_from_slice(&self.buf[bottom + center..bottom + center + 4]);
			set.0[32..].copy_from_slice(&self.buf[bottom + right..bottom + right + 4]);
		}

		// Bump the X coordinate unless we've reached the end of the line.
		if self.x + 1 < self.width { self.x += 1; }
		// Otherwise bump the Y (and shift the rows accordingly).
		else {
			self.x = 0;
			self.y += 1;
		}

		// Return the result!
		Some(set)
	}

	/// # Size Hint.
	///
	/// This hint should be "exact" as the iterator size is known at the
	/// outset.
	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for Nines<'a> {
	#[inline]
	fn len(&self) -> usize { self.width * self.height }
}



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

impl Default for Nine {
	#[inline]
	fn default() -> Self { Self([0; 36]) }
}

/// ## Getters.
impl Nine {
	/// # The Center Pixel's Red.
	const fn red(&self) -> u8 { self.0[16] }

	/// # The Center Pixel's Green.
	const fn green(&self) -> u8 { self.0[17] }

	/// # The Center Pixel's Blue.
	const fn blue(&self) -> u8 { self.0[18] }

	/// # The Center Pixel's Alpha.
	const fn alpha(&self) -> u8 { self.0[19] }

	/// # The Center Pixel's Flags.
	///
	/// This can be used to quickly determine whether or not the center pixel
	/// is visible — `alpha > 0` — and/or has alpha data — `alpha < 255`.
	const fn flags(&self) -> u8 {
		match self.alpha() {
			0 => FLAG_ALPHA,
			255 => FLAG_VISIBLE,
			_ => FLAG_ALPHA | FLAG_VISIBLE,
		}
	}

	/// # Has Invisible Pixels?
	///
	/// This returns true if any of the pixels in the set have an alpha value
	/// of zero.
	fn has_invisible(&self) -> bool {
		self.0.chunks_exact(4).any(|px| px[3] == 0)
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
			let mut avg = [
				num_integer::div_floor(r, weight) as u8,
				num_integer::div_floor(g, weight) as u8,
				num_integer::div_floor(b, weight) as u8,
				self.alpha(),
			];

			// Clamp values to keep them from straying too far afield.
			if self.alpha() != 0 {
				avg[0] = clamp(avg[0], self.red(), self.alpha());
				avg[1] = clamp(avg[1], self.green(), self.alpha());
				avg[2] = clamp(avg[2], self.blue(), self.alpha());
			}

			// Return if different!
			if self.0[16..20] == avg { None }
			else { Some(avg) }
		}
		else { None }
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
		let mut avg = [
			num_integer::div_floor(r, 9) as u8,
			num_integer::div_floor(g, 9) as u8,
			num_integer::div_floor(b, 9) as u8,
			self.alpha(),
		];

		// Clamp values to keep them from straying too far afield.
		if self.alpha() != 0 {
			avg[0] = clamp(avg[0], self.red(), self.alpha());
			avg[1] = clamp(avg[1], self.green(), self.alpha());
			avg[2] = clamp(avg[2], self.blue(), self.alpha());
		}

		// Return if different!
		if self.0[16..20] == avg { None }
		else { Some(avg) }
	}
}



#[allow(clippy::cast_possible_truncation)] // Values will be in range.
#[allow(clippy::similar_names)] // Weight and Height are quite different!
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
pub(super) fn clean_alpha(img: &mut Vec<u8>, width: usize, height: usize) {
	// First up, let's look for semi-transparent pixels appearing next to fully
	// transparent pixels, and average them up to create a suitable "default"
	// to apply to invisible pixels image-wide.
	let (r, g, b, weight) = Nines::new(img, width, height)
		.filter(|nine|
			(FLAG_SEMI_TRANSPARENT == nine.flags() & FLAG_SEMI_TRANSPARENT) &&
			nine.has_invisible()
		)
		.fold((0_u64, 0_u64, 0_u64, 0_u64), |mut acc, nine| {
			let weight = 256 - u64::from(nine.alpha());

			acc.0 += u64::from(nine.red()) * weight;
			acc.1 += u64::from(nine.green()) * weight;
			acc.2 += u64::from(nine.blue()) * weight;
			acc.3 += weight;

			acc
		});

	// We only need to continue if we found the pixels we were looking for.
	if 0 < weight {
		// Finish the average calculation to give us the neutral color.
		let neutral = [
			num_integer::div_floor(r, weight) as u8,
			num_integer::div_floor(g, weight) as u8,
			num_integer::div_floor(b, weight) as u8,
			0,
		];

		// Set all invisible pixels to said neutral color.
		img.chunks_exact_mut(4)
			.filter(|px| px[3] == 0)
			.for_each(|px| {
				px.copy_from_slice(&neutral);
			});

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
fn blur_alpha(img: &mut Vec<u8>, width: usize, height: usize) {
	// First compute a weighted average.
	let mut diff: Vec<(usize, [u8; 4])> = Nines::new(img, width, height)
		.enumerate()
		.filter_map(|(idx, nine)|
			if FLAG_ALPHA == nine.flags() & FLAG_ALPHA {
				Some((idx * 4, nine.weighted()?))
			}
			else { None }
		)
		.collect();

	// Apply the changes.
	diff.drain(..).for_each(|(idx, px)| {
		img[idx..idx + 4].copy_from_slice(&px);
	});

	// Now compute a straight average.
	diff.extend(
		Nines::new(img, width, height)
			.enumerate()
			.filter_map(|(idx, nine)|
				if FLAG_ALPHA == nine.flags() & FLAG_ALPHA {
					Some((idx * 4, nine.averaged()?))
				}
				else { None }
			)
	);

	// And apply it!
	diff.into_iter().for_each(|(idx, px)| {
		img[idx..idx + 4].copy_from_slice(&px);
	});
}

#[inline]
/// # Clamp Helper.
///
/// These lines get a little long...
fn clamp(px: u8, old: u8, a: u8) -> u8 {
	let (min, max) = premultiplied_minmax(old, a);
	px.max(min).min(max)
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

		// There should be 16 total iterations of a 4x4 "image".
		assert_eq!(Nines::new(&raw, 4, 4).count(), 16);

		let mut nine = Nines::new(&raw, 4, 4);

		// Test the first few bits.
		assert_eq!(
			nine.next(),
			Some(Nine([0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 16, 17, 18, 19, 20, 21, 22, 23]))
		);
		assert_eq!(
			nine.next(),
			Some(Nine([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27]))
		);
		assert_eq!(
			nine.next(),
			Some(Nine([4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31]))
		);
		assert_eq!(
			nine.next(),
			Some(Nine([8, 9, 10, 11, 12, 13, 14, 15, 12, 13, 14, 15, 8, 9, 10, 11, 12, 13, 14, 15, 12, 13, 14, 15, 24, 25, 26, 27, 28, 29, 30, 31, 28, 29, 30, 31]))
		);

		// Make sure rows shift correctly.
		assert_eq!(
			nine.next(),
			Some(Nine([0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 16, 17, 18, 19, 16, 17, 18, 19, 20, 21, 22, 23, 32, 33, 34, 35, 32, 33, 34, 35, 36, 37, 38, 39]))
		);
		assert_eq!(
			nine.next(),
			Some(Nine([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43]))
		);
		assert_eq!(
			nine.next(),
			Some(Nine([4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47]))
		);
		assert_eq!(
			nine.next(),
			Some(Nine([8, 9, 10, 11, 12, 13, 14, 15, 12, 13, 14, 15, 24, 25, 26, 27, 28, 29, 30, 31, 28, 29, 30, 31, 40, 41, 42, 43, 44, 45, 46, 47, 44, 45, 46, 47]))
		);

		// Jump to the end to make sure it stops at the right place.
		assert_eq!(
			nine.last(),
			Some(Nine([40, 41, 42, 43, 44, 45, 46, 47, 44, 45, 46, 47, 56, 57, 58, 59, 60, 61, 62, 63, 60, 61, 62, 63, 56, 57, 58, 59, 60, 61, 62, 63, 60, 61, 62, 63]))
		);
	}
}

