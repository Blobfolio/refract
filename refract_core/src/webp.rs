/*!
# `Refract`: Image `WebP`
*/

use crate::Image;
use crate::ImageKind;
use crate::Quality;
use crate::RefractError;
use crate::Refraction;
use dactyl::NiceU8;
use fyi_msg::Msg;
use std::ffi::OsStr;
use std::num::NonZeroU64;
use std::num::NonZeroU8;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;



#[derive(Debug, Clone)]
/// # `WebP`.
pub struct Webp<'a> {
	src: &'a PathBuf,
	src_size: NonZeroU64,

	dst: PathBuf,
	dst_size: Option<NonZeroU64>,
	dst_quality: Option<NonZeroU8>,

	tmp: PathBuf,
}

impl<'a> Webp<'a> {
	#[allow(trivial_casts)] // It is what it is.
	#[must_use]
	/// # New.
	pub fn new(src: &'a Image<'a>) -> Self {
		let stub: &[u8] = unsafe { &*(src.path().as_os_str() as *const OsStr as *const [u8]) };

		let mut out = Self {
			src: src.path(),
			src_size: src.size(),

			dst: PathBuf::from(OsStr::from_bytes(&[stub, b".webp"].concat())),
			dst_size: None,
			dst_quality: None,

			tmp: PathBuf::from(OsStr::from_bytes(&[stub, b".PROPOSED.webp"].concat())),
		};

		// Try lossless while we're here.
		if src.kind() == ImageKind::Png {
			let _res = out.make_lossless();
		}

		out
	}

	/// # Find the best!
	///
	/// ## Errors
	///
	/// Returns an error if no acceptable `WebP` can be found or if there are
	/// problems saving them.
	pub fn find(mut self) -> Result<Refraction, RefractError> {
		let prompt = Msg::custom(
			"WebP",
			208,
			&format!(
				"Does \x1b[1;95m{}\x1b[0m look good?",
				self.tmp
					.file_name()
					.ok_or(RefractError::InvalidImage)?
					.to_string_lossy(),
			)
		);

		let mut quality = Quality::default();
		while let Some(q) = quality.next() {
			match self.make_lossy(q) {
				Ok(size) => {
					if prompt.prompt() {
						quality.max(q);

						// Move it to the destination.
						std::fs::rename(&self.tmp, &self.dst)
							.map_err(|_| RefractError::Write)?;

						// Update the details.
						self.dst_quality = Some(q);
						self.dst_size = Some(size);
					}
					else {
						quality.min(q);
					}
				},
				Err(RefractError::TooBig) => {
					if let Some(x) = NonZeroU8::new(q.get().saturating_sub(1)) {
						quality.max(x);
					}
					else { return Err(RefractError::NoWebp); }
				},
				Err(e) => {
					return Err(e);
				},
			}
		}

		// Clean up.
		if self.tmp.exists() {
			let _res = std::fs::remove_file(&self.tmp);
		}

		if let Some((size, quality)) = self.dst_size.zip(self.dst_quality) {
			Ok(Refraction::new(self.dst, size, quality))
		}
		else {
			// Get rid of the distribution file if it exists.
			if self.dst.exists() {
				let _res = std::fs::remove_file(self.dst);
			}

			Err(RefractError::NoWebp)
		}
	}

	/// # Make Lossless.
	fn make_lossless(&mut self) -> Result<(), RefractError> {
		// Clear the temporary file, if any.
		if self.tmp.exists() {
			std::fs::remove_file(&self.tmp).map_err(|_| RefractError::Write)?;
		}

		let status = Command::new("cwebp")
			.args(&[
				OsStr::new("-quiet"),
				OsStr::new("-lossless"),
				OsStr::new("-z"),
				OsStr::new("9"),
				OsStr::new("-q"),
				OsStr::new("100"),
				OsStr::new("-mt"),
				self.src.as_os_str(),
				OsStr::new("-o"),
				self.tmp.as_os_str(),
			])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|_| RefractError::Write)?;

		// Did it not work?
		if ! status.success() || ! self.tmp.exists() {
			return Err(RefractError::Write);
		}

		// Find the file size.
		let size = NonZeroU64::new(std::fs::metadata(&self.tmp).map_or(0, |m| m.len()))
			.ok_or(RefractError::Write)?;

		if size < self.src_size {
			// Move it to the destination.
			std::fs::rename(&self.tmp, &self.dst).map_err(|_| RefractError::Write)?;

			// Update the details.
			self.dst_quality = Some(unsafe { NonZeroU8::new_unchecked(100) });
			self.dst_size = Some(size);

			Ok(())
		}
		else {
			Err(RefractError::TooBig)
		}
	}

	/// # Make Lossy.
	fn make_lossy(&self, quality: NonZeroU8) -> Result<NonZeroU64, RefractError> {
		// Clear the temporary file, if any.
		if self.tmp.exists() {
			std::fs::remove_file(&self.tmp).map_err(|_| RefractError::Write)?;
		}

		let status = Command::new("cwebp")
			.args(&[
				OsStr::new("-quiet"),
				OsStr::new("-q"),
				OsStr::new(NiceU8::from(quality).as_str()),
				OsStr::new("-m"),
				OsStr::new("6"),
				OsStr::new("-pass"),
				OsStr::new("10"),
				OsStr::new("-mt"),
				self.src.as_os_str(),
				OsStr::new("-o"),
				self.tmp.as_os_str(),
			])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|_| RefractError::Write)?;

		// Did it not work?
		if ! status.success() || ! self.tmp.exists() {
			return Err(RefractError::Write);
		}

		// Find the file size.
		let size = NonZeroU64::new(std::fs::metadata(&self.tmp).map_or(0, |m| m.len()))
			.ok_or(RefractError::Write)?;

		// It has to be smaller than what we've already chosen.
		if let Some(dsize) = self.dst_size {
			if size < dsize { Ok(size) }
			else { Err(RefractError::TooBig) }
		}
		// It has to be smaller than the source.
		else if size < self.src_size { Ok(size) }
		else { Err(RefractError::TooBig) }
	}
}
