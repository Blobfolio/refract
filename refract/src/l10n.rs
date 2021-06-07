/*!
# `Refract GTK` - Language Bits
*/

use dactyl::NiceU64;
use std::borrow::Cow;
 


/// # Inflect.
///
/// Return the singular or plural version of a noun given the count.
pub(super) fn inflect<T>(len: usize, singular: T, plural: T) -> String
where T: AsRef<str> {
	let size = NiceU64::from(len);
	format!(
		"{} {}",
		size.as_str(),
		if len == 1 { singular.as_ref() }
		else { plural.as_ref() }
	)
}

/// # Oxford Join.
///
/// Separate entries with Oxford-style commas as needed.
pub(super) fn oxford_join<'a, T>(set: &'a [T], glue: &str) -> Cow<'a, str>
where T: AsRef<str> {
	match set.len() {
		0 => Cow::Borrowed(""),
		1 => Cow::Borrowed(set[0].as_ref()),
		2 => Cow::Owned(format!("{} {} {}", set[0].as_ref(), glue, set[1].as_ref())),
		n => {
			// Join all but the last item with trailing commas.
			let mut base: String = set[..n - 1].iter().fold(String::new(), |mut out, s| {
				out.push_str(s.as_ref());
				out.push_str(", ");
				out
			});

			// Add the glue, a space, and the last item.
			base.push_str(glue);
			base.push(' ');
			base.push_str(set[n - 1].as_ref());
			Cow::Owned(base)
		}
	}
}
