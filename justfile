##
# Development Recipes
#
# This justfile is intended to be run from inside a Docker sandbox:
# https://github.com/Blobfolio/righteous-sandbox
#
# docker run \
#	--rm \
#	-v "{{ invocation_directory() }}":/share \
#	-it \
#	--name "righteous_sandbox" \
#	"righteous/sandbox:debian"
#
# Alternatively, you can just run cargo commands the usual way and ignore these
# recipes.
##

pkg_id      := "refract"
pkg_name    := "Refract"
pkg_dir1    := justfile_directory() + "/refract"
pkg_dir2    := justfile_directory() + "/refract_core"

cargo_dir   := "/tmp/" + pkg_id + "-cargo"
cargo_bin   := cargo_dir + "/release/" + pkg_id
data_dir    := "/tmp/bench-data"
doc_dir     := justfile_directory() + "/doc"
release_dir := justfile_directory() + "/release"



#export RUSTFLAGS := "-Ctarget-cpu=x86-64-v3 -Cllvm-args=--cost-kind=throughput -Clinker-plugin-lto -Clink-arg=-fuse-ld=lld"
#export CC        := "clang"
#export CXX       := "clang++"
#export CFLAGS    := `llvm-config --cflags` + " -march=x86-64-v3 -Wall -Wextra -flto"
#export CXXFLAGS  := `llvm-config --cxxflags` + " -march=x86-64-v3 -Wall -Wextra -flto"
#export LDFLAGS   := `llvm-config --ldflags` + " -fuse-ld=lld -flto"



# Build Release!
@build:
	cargo build \
		--bin "{{ pkg_id }}" \
		-p "{{ pkg_id }}" \
		--release \
		--target-dir "{{ cargo_dir }}"


# Build Debian package!
@build-deb: clean credits build
	# cargo-deb doesn't support target_dir flags yet.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	mv "{{ cargo_dir }}" "{{ justfile_directory() }}/target"

	# Build the deb.
	cargo-deb \
		--no-build \
		--quiet \
		-p {{ pkg_id }} \
		-o "{{ release_dir }}"

	just _fix-chown "{{ release_dir }}"
	mv "{{ justfile_directory() }}/target" "{{ cargo_dir }}"


@clean:
	# Most things go here.
	[ ! -d "{{ cargo_dir }}" ] || rm -rf "{{ cargo_dir }}"

	# But some Cargo apps place shit in subdirectories even if
	# they place *other* shit in the designated target dir. Haha.
	[ ! -d "{{ justfile_directory() }}/target" ] || rm -rf "{{ justfile_directory() }}/target"
	[ ! -d "{{ pkg_dir1 }}/target" ] || rm -rf "{{ pkg_dir1 }}/target"
	[ ! -d "{{ pkg_dir2 }}/target" ] || rm -rf "{{ pkg_dir2 }}/target"


# Clippy.
@clippy:
	clear

	fyi task "All Features"
	cargo clippy \
		--release \
		--workspace \
		--all-features \
		--target-dir "{{ cargo_dir }}"

	just _clippy avif
	just _clippy avif,jpeg
	just _clippy avif,jpeg,jxl
	just _clippy avif,jpeg,jxl,png
	just _clippy avif,jpeg,jxl,png,webp

	just _clippy jpeg
	just _clippy jpeg,jxl
	just _clippy jpeg,jxl,png
	just _clippy jpeg,jxl,png,webp

	just _clippy jxl
	just _clippy jxl,png
	just _clippy jxl,png,webp

	just _clippy png
	just _clippy png,webp

	just _clippy webp

@_clippy FEATURES:
	fyi task "Features: {{ FEATURES }}"
	cargo clippy \
		--release \
		-p refract_core \
		--no-default-features \
		--features "{{ FEATURES }}" \
		--target-dir "{{ cargo_dir }}"


# Generate CREDITS.
@credits:
	cargo bashman -m "{{ pkg_dir1 }}/Cargo.toml" -t x86_64-unknown-linux-gnu
	just _fix-chown "{{ justfile_directory() }}/CREDITS.md"


# Test Run.
@run +ARGS:
	cargo run \
		--bin "{{ pkg_id }}" \
		--release \
		--target-dir "{{ cargo_dir }}" \
		-- {{ ARGS }}


# Unit tests!
@test:
	clear
	cargo test \
		--workspace \
		--target-dir "{{ cargo_dir }}"
	cargo test \
		--release \
		--workspace \
		--target-dir "{{ cargo_dir }}"


# Get/Set version.
version:
	#!/usr/bin/env bash
	set -e

	# Current version.
	_ver1="$( tomli query -f "{{ pkg_dir1 }}/Cargo.toml" package.version | \
		sed 's/[" ]//g' )"

	# Find out if we want to bump it.
	set +e
	_ver2="$( whiptail --inputbox "Set {{ pkg_name }} version:" --title "Release Version" 0 0 "$_ver1" 3>&1 1>&2 2>&3 )"

	exitstatus=$?
	if [ $exitstatus != 0 ] || [ "$_ver1" = "$_ver2" ]; then
		exit 0
	fi
	set -e

	# Set the release version!
	tomli set -f "{{ pkg_dir1 }}/Cargo.toml" -i package.version "$_ver2"
	tomli set -f "{{ pkg_dir2 }}/Cargo.toml" -i package.version "$_ver2"

	fyi success "Set version to $_ver2."


# Init dependencies.
@_init:
	# Nothing just now.


# Fix file/directory permissions.
@_fix-chmod PATH:
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type f -exec chmod 0644 {} +
	[ ! -e "{{ PATH }}" ] || find "{{ PATH }}" -type d -exec chmod 0755 {} +


# Fix file/directory ownership.
@_fix-chown PATH:
	[ ! -e "{{ PATH }}" ] || chown -R --reference="{{ justfile() }}" "{{ PATH }}"
