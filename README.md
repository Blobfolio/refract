# Refract GTK

[![deps.rs](https://deps.rs/repo/github/blobfolio/refract/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/refract)
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/refract/issues)

Refract is a guided image conversion tool written in [Rust](https://www.rust-lang.org/) for x86-64 Linux systems with [GTK](https://www.gtk.org/).

It takes [JPEG](https://en.wikipedia.org/wiki/JPEG) and [PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics) image sources and produces [AVIF](https://en.wikipedia.org/wiki/AV1#AV1_Image_File_Format_(AVIF)), [JPEG XL](https://en.wikipedia.org/wiki/JPEG_XL), and [WebP](https://en.wikipedia.org/wiki/WebP) clones.

<img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen0.png" width="30%" alt="The start screen. Nice and clean."></img> <img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen1.png" width="30%" alt="Viewing a PNG source image."></img> <img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen2.png" width="30%" alt="Previewing a WebP candidate to Discard or Keep."></img> 

The program is named after — and works like — eye doctor Refraction Tests. It generates candidate images at various qualities, asking at each step how it looks, and uses the feedback (you provide) to arrive at the smallest possible "acceptable" output.

Hence "guided".

The beauty of this approach is that it moots the need for exhaustive testing. Refract's quality stepping works by selecting the mid-point between moving min/max boundaries. Similar to a binary search, each answer you provide halves the range of possibilities, allowing the final, perfect result to be discovered in just 5-10 steps instead of 100+.



## Why?

Every image is different. The idea of a "Magic Bullet" format is a myth.

If you want to truly maximize the quality and size of next-gen copies, you cannot rely on hardcoded constants or fancy [SSIM](https://en.wikipedia.org/wiki/Structural_similarity) analysis. That will result in frequent over- or under-compression, and some images will just come out looking… worse.

You have to actually _use your eyes_. And you have to pay attention to the resulting file sizes. Sometimes newer formats will result in _larger_ output than the original source, defeating the purpose. Haha.

While you can do all of this manually — running multiple programs hundreds of times for each and every source you want to convert — that would be incredibly tedious and easy to screw up.

Refract helps make that manual process _less_ tedious and _more_ foolproof.

It automatically uses the strongest (slowest) possible compression settings for each format, keeps track of file sizes and qualities along the way, can process inputs en masse, and reduces the number of conversion tests by around 90%.

Should you use it for every image ever?

No, probably not.

The next generation formats, particularly AVIF and JPEG XL, require a lot of computation to eek out their extra byte savings. All those minutes will add up quickly.

But if you're looking to obsessively optimize a small project or single web page, Refract is the way to go!



## Features

| Format | Decoding (Input/Display) | Encoding (Output) |
| ------ | -------- | -------- |
| JPEG | Yes, except CMYK and 16-bit lossless. ||
| PNG  | Yes* ||
| AVIF | Yes | Lossless, lossy, `RGB`, and `YCbCr` |
| JPEG XL | Yes* | Lossless, lossy. |
| WebP | Yes* | Lossless, lossy. |

*Refract does not support animated images. Without going too far down _that_ rabbit hole, let's just say that if GIF can't handle the job, it should be a video, not an image.

In other words, Refract takes JPEG and PNG sources — either individual files or entire directory trees — and turns them into AVIF, JPEG XL, and/or WebP outputs.

Refract implements [`libavif`](https://github.com/AOMediaCodec/libavif), [`libjxl`](https://github.com/libjxl/libjxl), and [`libwebp`](https://chromium.googlesource.com/webm/libwebp/) directly. This not only ensures full standards compliance and feature/performance parity with each format's official conversion tools — `avifenc`, `cjxl`, and `cwebp` respectively — it also means you don't need any of that crap separately installed to use it.

All conversion takes place at Pixel Level and is intended for displays with an sRGB color space (e.g. web browsers). Gamma correction, color profiles, and other metadata are ignored and stripped out when saving next-gen copies.



## Usage

Refract is pretty straightforward:

1. Tweak the settings — via the `Settings` menu — as desired.
2. Load a single image or an entire directory. You can either use the links in the `File` menu, or drag-and-drop images straight onto the window from your file browser.
3. Sit back and wait for any feedback or save prompts.

For best results, be sure to optimize your input sources before re-encoding them with Refract. (The CLI tool [flaca](https://github.com/Blobfolio/flaca) is great for this, and fully automatic.)

For keyboard aficionados, the following hot-keys may be used:

| Action | Key(s) |
| ------ | ------ |
| Open File | `CTRL + o` |
| Open Directory | `SHIFT + CTRL + o` |
| Toggle Dark Mode | `CTRL + n` |
| Toggle A/B View | `SPACE` |
| Discard Candidate | `d` |
| Keep Candidate | `k` |



## CLI Usage

Refract is a _graphical_ program, but when launching from the command line, you can override the default settings and/or queue up images to re-encode.

```bash
refract [FLAGS] [OPTIONS] <PATH(S)>...
```

| Flag | Description |
| ---- | ----------- |
| `-h` / `--help` | Print help information and exit. |
| `-V` / `--version` | Print version information and exit. |
| `--no-avif` | Skip AVIF encoding. |
| `--no-jxl` | Skip JPEG-XL encoding. |
| `--no-webp` | Skip WebP Encoding. |
| `--no-lossless` | Skip lossless encoding passes. |
| `--no-lossy` | Skip lossy encoding passes. |
| `--no-ycbcr` | Skip AVIF YCbCr encoding passes. |

Note: The flags only affect the initial program state. All settings can still be managed through the program's dropdown menus after launch.

| Option | Description |
| ------ | ----------- |
| `-l` / `--list` | Read (absolute) image and/or directory paths from this text file, one path per line. This is equivalent to specifying the same paths as trailing arguments, but can be cleaner if there are lots of them. |

When image and/or directory paths are passed as trailing arguments (`<PATH(S)>...`), and/or the `-l`/`--list` option is used, Refract will start crunching all valid sources as soon as the program launches.



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [release page](https://github.com/Blobfolio/refract/releases/latest).

(Arch Linux users can probably use the `.deb` too, but may need to adjust the icon and `.desktop` paths to match where your system likes to keep those things.)

While specifically written for use on x86-64 Linux systems, both Rust and GTK are cross-platform, so you may well be able to build it from source on other 64-bit Unix systems using `Cargo`:

```bash
# Clone the repository.
git clone https://github.com/Blobfolio/refract.git

# Move into the directory.
cd refract

# Build with Cargo. Feel free to add other build flags as desired.
cargo build --release
```

Cargo _will_ handle the entire build process for you, however many of Refract's dependencies have heavy `build.rs` scripts requiring additional system libraries. (Who'd have thought image decoders and encoders were complicated?!)

At a minimum, you'll need up-to-date versions of:

* Cmake
* `gcc`/`g++`
* Git
* Make
* NASM
* Ninja
* Rust (>= `1.59`)/Cargo

GTK is a whole other monster, requiring the `-dev` packages for (at least) ATK, Cairo, GDK, GLIB, GTK, Pango, and Pixbuf. Thankfully, many distributions offer meta packages to make GTK dependency resolution easier. On Debian Bullseye, for example, installing `librust-gtk-dev` and `librust-gdk-dev` should just about cover everything.

[This post](https://github.com/Blobfolio/refract/issues/3#issuecomment-1086924244) provides a good breakdown of how to set up a minimal Docker build environment for Refract.

If you end up building Refract on a non-Debian system — Red Hat, MacOS, etc. — please let us know what that setup looked like so we can update the docs. Users of those systems will no doubt appreciate it. :)



## License

See also: [CREDITS.md](CREDITS.md)

Copyright © 2022 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004
    
    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>
    
    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.
    
    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
    
    0. You just DO WHAT THE FUCK YOU WANT TO.
