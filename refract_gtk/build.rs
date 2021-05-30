/*!
# `Refract GTK` - Build

This is used to compile a resource bundle of the various assets that need to
be pulled into GTK.
*/

use std::{
	ffi::OsStr,
	process::{
		Command,
		Stdio,
	},
};

/// # Bundle Resources.
pub fn main() {
	println!("cargo:rerun-if-changed=skel");

	// The directory with all the files.
	let skel_dir = std::fs::canonicalize(concat!(env!("CARGO_MANIFEST_DIR"), "/skel"))
		.expect("Missing /skel directory.");

	// The input resource manifest.
	let in_file = {
		let mut dir = skel_dir.clone();
		dir.push("resources.xml");
		dir
	};
	assert!(in_file.is_file(), "Missing resources.xml");

	// The output location for the resource manifest.
	let out_file = {
		let mut dir = std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
			.expect("Missing OUT_DIR.");
		dir.push("resources.gresource");
		dir
	};

	// Build it!
	if ! Command::new("glib-compile-resources")
		.current_dir(&skel_dir)
		.args(&[
			OsStr::new("--target"),
			out_file.as_os_str(),
			in_file.as_os_str(),
		])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.map_or(false, |s| s.success()) {
			panic!("Unable to bundle resources with glib-compile-resources; is GLIB installed?");
		}

	assert!(out_file.is_file(), "Missing the resource bundle.");
}
