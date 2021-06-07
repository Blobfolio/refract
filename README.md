# Refract GTK

Refract is a guided image conversion tool written in [Rust](https://www.rust-lang.org/) for x86-64 Linux systems with [GTK](https://www.gtk.org/).

It takes [JPEG](https://en.wikipedia.org/wiki/JPEG) and [PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics) image sources and produces [AVIF](https://en.wikipedia.org/wiki/AV1#AV1_Image_File_Format_(AVIF)), [JPEG XL](https://en.wikipedia.org/wiki/JPEG_XL), and [WebP](https://en.wikipedia.org/wiki/WebP) clones.

<img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen0.png" width="30%" alt="The start screen. Nice and clean."></img> <img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen1.png" width="30%" alt="Viewing a PNG source image."></img> <img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen2.png" width="30%" alt="Previewing a WebP candidate to Discard or Keep."></img> 

The program is named after — and works like — eye doctor Refraction Tests. It generates candidate images at various qualities, asking at each step how it looks, and uses that feedback (you provide) to arrive at the smallest possible "acceptable" output.

Hence "guided".

The beauty of this approach is that it moots the need for exhaustive testing. Refract's quality stepping works by selecting the mid-point between moving min/max boundaries. Each answer you provide adjusts the range, allowing the final, perfect result to be discovered in just 5-10 steps instead of 100+.



## Why?

Every image is different. The idea of a "Magic Bullet" format is a myth.

If you want to truly maximize the quality and size of next-gen copies, you cannot rely on hardcoded constants or fancy [SSIM](https://en.wikipedia.org/wiki/Structural_similarity) analysis. That will result in frequent over- or under-compression, and some images will just come out looking… worse.

You have to actually _use your eyes_. And you have to pay attention to the resulting file sizes. Sometimes newer formats will result in _larger_ output than the original source, defeating the purpose!

While you can do all of this manually — running multiple programs hundreds of times for each and every source you want to convert — that would be incredibly tedious and easy to screw up.

Refract helps makes that manual process _less_ tedious and _more_ foolproof.

It automatically uses the strongest (slowest) possible compression settings for each format, keeps track of file sizes and qualities along the way, can process inputs en masse, and reduces the number of conversion tests by around 90%.

Should you use it for every image ever?

No, probably not.

The next generation formats, particularly AVIF and JPEG XL, require a lot of computation to eek out their extra byte savings. All those minutes will add up quickly.

But if you're looking to obsessively optimize a small project or single web page, Refract is the way to go!



## Features

| Format | Decoding (Input/Display) | Encoding (Output) |
| ------ | -------- | -------- |
| JPEG | Yes, except CMYK. ||
| PNG  | Yes* ||
| AVIF | Yes | Lossless, lossy, `RGB`, and `YCbCr` |
| JPEG XL | Yes* | Lossless, lossy. |
| WebP | Yes* | Lossless, lossy. |

** Refract does not support animated images. Without going too far down _that_ rabbit hole, let's just say that if GIF can't handle the job, it should be a video, not an image.

In other words, Refract takes JPEG and PNG sources — either individual files or entire directory trees — and turns them into AVIF, JPEG XL, and/or WebP outputs.

Refract implements [`libavif`](https://github.com/AOMediaCodec/libavif), [`libjxl`](https://gitlab.com/wg1/jpeg-xl), and [`libwebp`](https://chromium.googlesource.com/webm/libwebp/) directly. This not only ensures full standards compliance and feature/performance parity with each format's official conversion tools — `avifenc`, `cjxl`, and `cwebp` respectively — it also means you don't need any of that crap separately installed to use it!

All conversion takes place at Pixel Level and is intended for displays with an sRGB color space (e.g. web browsers). Gamma correction, color profiles, and other metadata are ignored and stripped out when saving next-gen copies.



## Usage

Refract is pretty straightforward:

1. Tweak the settings — via the `Settings` menu — as desired.
2. Load a single image or an entire directory.
3. Sit back and wait for any feedback or save prompts.

For best results, be sure to optimize your input sources before re-encoding them with Refract.

For keyboard afficionados, the following hot-keys may be used:

| Action | Key(s) |
| ------ | ------ |
| Open File | `CTRL + o` |
| Open Directory | `SHIFT + CTRL + o` |
| Toggle View | `SPACE` |
| Discard Candidate | `d` |
| Keep Candidate | `k` |



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [release page](https://github.com/Blobfolio/refract/releases/latest).

(Arch Linux users can probably use the `.deb` too, but may need to adjust the icon and `.desktop` paths to match where your system likes to keep those things.)

While specifically written for use on x86-64 Linux systems, both Rust and GTK are cross-platform, so you may well be able to build it from source on other 64-bit Unix systems using `Cargo`:

```
# Clone the repository.
git clone https://github.com/Blobfolio/refract.git

# Move into the directory.
cd refract

# Build with Cargo. Feel free to add other build flags as desired.
cargo build \
    --bin refract \
    -p refract \
    --release
```

Cargo _will_ handle the entire build process for you, however many of Refract's dependencies have heavy `build.rs` scripts requiring additional system libraries.

At a minimum, you'll need up-to-date versions of:

* Clang
* Cmake
* GCC
* Git
* Make
* NASM
* Ninja

You'll also need the `-dev` packages for all of the GTK dependencies, including ATK, Cairo, GDK, GLIB, Pango, and Pixbuf. Thankfully, many distributions offer meta packages to make the GTK dependency installation process easier. On Debian Buster, for example, installing `librust-gtk-dev` and `librust-gdk-dev` should just about cover everything.

This list is (probably) non-exhaustive. If you find you need something else, open a ticket so I can update the list!

Likewise, if you try building on Mac, please let me know how it goes!



## License

See also: [CREDITS.md](CREDITS.md)

Copyright © 2021 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

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
