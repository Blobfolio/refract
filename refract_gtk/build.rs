/*!
# `Refract GTK` - Build

This is used to compile a resource bundle of the various assets that need to
be pulled into GTK.
*/

use std::{
	collections::HashMap,
	ffi::OsStr,
	fs::File,
	io::Write,
	path::PathBuf,
	process::{
		Command,
		Stdio,
	},
};
use toml::Value;
use version_compare::Version;



/// # Build!
pub fn main() {
	println!("cargo:rerun-if-changed=Cargo.toml");
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
///
/// This compiles a list of crates used as direct dependencies (to both GTK and
/// core, since both are ours).
///
/// This data gets used inside the Help > About dialogue.
fn _credits() {
	// Parse the lock file.
	let lock_toml = _man_path("Cargo.lock")
		.or_else(|| _man_path("../Cargo.lock"))
		.and_then(|p| std::fs::read_to_string(p).ok())
		.and_then(|p| p.parse::<Value>().ok())
		.expect("Unable to parse Cargo.lock.");

	// Build a list of all package dependencies by crate.
	let mut raw: HashMap<String, (String, Vec<String>)> = HashMap::new();
	lock_toml.get("package").and_then(Value::as_array).expect("Unable to parse Cargo.lock")
		.iter()
		.for_each(|entry| {
			let package = entry.as_table().expect("Malformed package entry.");
			let name = package.get("name")
				.and_then(Value::as_str)
				.map(String::from)
				.expect("Missing package name.");
			let version = package.get("version")
				.and_then(Value::as_str)
				.map(String::from)
				.expect("Missing package version.");
			let deps = _credits_deps(package.get("dependencies"));

			// It is already listed. Keep the more recent of the two.
			if let Some(existing) = raw.get(&name) {
				if Version::from(&existing.0) < Version::from(&version) {
					raw.remove(&name);
					raw.insert(name, (version, deps));
				}
			}
			else {
				raw.insert(name, (version, deps));
			}
		});

	// Make sure we have *this* entry.
	assert!(raw.contains_key("refract-gtk") && raw.contains_key("refract_core"), "Unable to parse Cargo.lock.");

	// Build a list of direct package dependencies for *this* crate.
	let mut list: Vec<String> = _credits_deps_formatted("refract-gtk", &raw);
	list.extend(_credits_deps_formatted("refract_core", &raw));
	list.sort();
	list.dedup();

	// Save them as a slice value!
	let mut file = _out_path("about-credits.txt")
		.and_then(|p| File::create(p).ok())
		.expect("Missing OUT_DIR.");

	file.write_fmt(format_args!("&[{}]", list.join(", ")))
		.and_then(|_| file.flush())
		.expect("Unable to save credits.");
}

/// # Credit Dependency Array.
///
/// This parses the dependencies for a given crate. There may not be any, in
/// which case an empty vector is returned.
fn _credits_deps(val: Option<&Value>) -> Vec<String> {
	if let Some(arr) = val.and_then(Value::as_array) {
		let mut arr: Vec<String> = arr.iter()
			.filter_map(Value::as_str)
			.map(String::from)
			.collect();

		arr.sort();
		arr.dedup();
		arr
	}
	else { Vec::new() }
}

/// # Credit Dependency Formatted.
///
/// This formats direct dependencies as a "Name Version URL" string. Because of
/// the limited scope, we can assume all entries exist on `crates.io`.
fn _credits_deps_formatted(key: &str, map: &HashMap<String, (String, Vec<String>)>)
-> Vec<String> {
	if let Some(deps) = map.get(key) {
		deps.1.iter()
			// Ignore our build dependencies, etc.
			.filter(|x|
				"argyle" != x.as_str() &&
				"refract_core" != x.as_str() &&
				"toml" != x.as_str() &&
				"version-compare" != x.as_str()
			)
			.filter_map(|name| map.get(name).map(|entry| format!(
				"\"{} v{} https://crates.io/crates/{}\"",
				name,
				entry.0,
				name,
			)))
			.collect()
	}
	else { Vec::new() }
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
