/*!
# `Refract GTK` - Build

This is used to compile a resource bundle of the various assets that need to
be pulled into GTK.
*/

use std::{
	ffi::OsStr,
	fs::File,
	path::PathBuf,
	process::{
		Command,
		Stdio,
	},
};

/// # Build!
pub fn main() {
	println!("cargo:rerun-if-changed=skel");

	_credits();
	_resources();
}

/// # Build Resource Bundle.
fn _resources() {
	// The directory with all the files.
	let skel_dir = _man_path("skel").expect("Missing /skel directory.");

	// The input resource manifest.
	let in_file = _man_path("skel/resources.xml").expect("Missing resources.xml");

	// The output location for the resource manifest.
	let out_file = _out_path("resources.gresource").expect("Missing OUT_DIR.");

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

	// Make sure that created the file.
	assert!(out_file.is_file(), "Missing the resource bundle.");
}

/// # Build Credits.
fn _credits() {
	use std::io::Write;

	// Parse the GTK and Core manifests.
	let man1 = _man_path("Cargo.toml")
		.and_then(|p| std::fs::read(p).ok())
		.and_then(|d| cargo_toml::Manifest::from_slice(&d).ok())
		.expect("Unable to parse refract_gtk manifest.");
	let man2 = _man_path("../refract_core/Cargo.toml")
		.and_then(|p| std::fs::read(p).ok())
		.and_then(|d| cargo_toml::Manifest::from_slice(&d).ok())
		.expect("Unable to parse refract_core manifest.");

	// Tease out the direct dependencies.
	let mut deps: Vec<String> = man1.dependencies.keys()
		.chain(man2.dependencies.keys())
		.filter_map(|k|
			if k == "argyle" || k == "refract_core" { None }
			else { Some(format!("\"{} https://crates.io/crates/{}\"", k, k)) }
		)
		.collect();

	deps.sort();
	deps.dedup();

	// Save them as a slice value!
	let mut file = _out_path("about-credits.txt")
		.and_then(|p| File::create(p).ok())
		.expect("Missing OUT_DIR.");

	file.write_fmt(format_args!("&[{}]", deps.join(", ")))
		.and_then(|_| file.flush())
		.expect("Unable to save credits.");
}

/// # Manifest Path.
///
/// Return a path relative to the manifest directory.
fn _man_path(file: &str) -> Option<PathBuf> {
	let mut dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).ok()?;
	dir.push(file);
	Some(dir).filter(|x| x.exists())
}

/// # Output Path.
///
/// Return a path relative to the output directory.
fn _out_path(file: &str) -> Option<PathBuf> {
	let mut dir = std::fs::canonicalize(std::env::var("OUT_DIR").ok()?).ok()?;
	dir.push(file);
	Some(dir)
}
