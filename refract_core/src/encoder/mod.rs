/*!
# `Refract`: Encoders.
*/

pub(super) mod webp;
pub(super) mod avif;




#[macro_export(local_inner_macros)]
/// # Helper: Find the Best Image.
macro_rules! impl_find {
	($name:literal, $error:expr) => (
		/// # Find the best!
		///
		/// This will generate lossy image copies in a loop with varying
		/// qualities, asking at each step whether or not the image looks OK.
		/// In most cases, an answer should be found in 5-10 steps.
		///
		/// If an acceptable candidate is found — based on user feedback and
		/// file size comparisons — it will be saved by appending the new
		/// format's extension to the original source path.
		///
		/// If no candidate is found or it is too big, no image will be saved.
		///
		/// Note: this method is consuming; the instance will not be usable
		/// afterward.
		///
		/// ## Errors
		///
		/// Returns an error if no acceptable candidate can be found or if
		/// there are problems saving them.
		pub fn find(mut self) -> Result<$crate::Refraction, $crate::RefractError> {
			let prompt = fyi_msg::Msg::custom(
				$name,
				208,
				&std::format!(
					"Does \x1b[1;95m{}\x1b[0m look good?",
					self.tmp
						.file_name()
						.ok_or($crate::RefractError::InvalidImage)?
						.to_string_lossy(),
				)
			);

			let mut quality = $crate::Quality::default();
			while let Some(q) = quality.next() {
				match self.make_lossy(q) {
					Ok(size) => {
						// It looks fine, so we can set the ceiling to the
						// level we just tested.
						if prompt.prompt() {
							quality.set_max(q);

							// Move it to the destination.
							std::fs::rename(&self.tmp, &self.dst)
								.map_err(|_| $crate::RefractError::Write)?;

							// Update the details.
							self.dst_quality = Some(q);
							self.dst_size = Some(size);
						}
						// It looks bad, so we can set the floor to the level
						// we just tested.
						else {
							quality.set_min(q);
						}
					},
					// The image came out too big; the ceiling must therefore
					// be (at least) quality-1.
					Err($crate::RefractError::TooBig) => {
						if let Some(x) = std::num::NonZeroU8::new(q.get().saturating_sub(1)) {
							quality.set_max(x);
						}
						else { return Err($error); }
					},
					// Any other kind of error indicates a problem with the
					// whole process; we have to abort.
					Err(e) => {
						return Err(e);
					},
				}
			}

			// Clean up.
			if self.tmp.exists() {
				let _res = std::fs::remove_file(&self.tmp);
			}

			// We found a candidate!
			if let Some((size, quality)) = self.dst_size.zip(self.dst_quality) {
				Ok($crate::Refraction::new(self.dst, size, quality))
			}
			// Oops, nothing doing.
			else {
				// Get rid of the distribution file if it exists.
				if self.dst.exists() {
					let _res = std::fs::remove_file(self.dst);
				}

				Err($error)
			}
		}
	);
}
