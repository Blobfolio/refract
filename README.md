# Refract

[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/refract/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/refract/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/refract/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/refract)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/refract/issues)

Refract is a cross-platform[\*](#installation) guided image conversion tool.

It takes [JPEG](https://en.wikipedia.org/wiki/JPEG) and [PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics) image sources and produces [AVIF](https://en.wikipedia.org/wiki/AV1#AV1_Image_File_Format_(AVIF)), [JPEG XL](https://en.wikipedia.org/wiki/JPEG_XL), and [WebP](https://en.wikipedia.org/wiki/WebP) copies.

<img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen0.png" width="45%" alt="Viewing a PNG source."></img> <img src="https://github.com/Blobfolio/refract/raw/master/skel/gallery/screen1.png" width="45%" alt="Viewing a (crappy) WebP copy."></img>

The program is named for and works something like an optometrist's refraction test, presenting a series of feedback-driven candidate images — what looks better, this… or this? This… or this? — until the subjective "best" option is found (or found to be impossible).

Hence "guided".

The beauty of this sort of approach is it moots the need for exhaustive testing.

Every Yay and Nay you provide halve the number of possibilities remaining to be tested. The ideal copy, whatever and wherever it might be, can be found in a handful of steps instead of a hundred.



## Why?

Every image is different.

There is no such thing as a one-size-fits-all quality setting, or even a one-size-fits-all image format.

Whether you're looking for perfect copies or merely passable ones, the only way to be sure you're not producing over- or under-compressed images is to _use your eyes_.

Done manually, you'd need to use a lot more — your brain, for starters — but thankfully most of the rest of the process _can_ be automated.

That's where refract comes in.

It keeps track of the details so you don't have to.



## Features

| Format | Decoding (Input/Display) | Encoding (Output) |
| ------ | -------- | -------- |
| JPEG | Yes, except CMYK and 16-bit lossless. ||
| PNG  | Yes ||
| AVIF | Yes | Lossless, lossy, `RGB`, and `YCbCr` |
| JPEG XL | Yes* | Lossless, lossy. |
| WebP | Yes* | Lossless, lossy. |

In short, Refract takes JPEG and PNG sources — either individual files or entire directory trees — and turns them into AVIF, JPEG XL, and/or WebP outputs.

Refract implements [`libavif`](https://github.com/AOMediaCodec/libavif), [`libjxl`](https://github.com/libjxl/libjxl), and [`libwebp`](https://chromium.googlesource.com/webm/libwebp/) directly. This not only ensures full standards compliance and feature/performance parity with each format's official conversion tools — `avifenc`, `cjxl`, and `cwebp` respectively — it also means you don't need any of that crap separately installed to use it.

All conversion takes place at Pixel Level and is intended for displays with an sRGB color space (e.g. web browsers). Gamma correction, color profiles, and other metadata are ignored and stripped out when saving next-gen copies.

All conversions are performed using the maximum/slowest (general) encoder settings, ensuring the smallest possible output. Refract also explicitly tracks the input and output file sizes to save you having to review counter-productive combinations.



## Usage

Refract is pretty straightforward:

0. Open the program;
1. Choose the output format(s) and tweak any other settings you want;
2. Choose one or more JPEG/PNG source files to crunch;
3. Sit back and wait for the feedback prompts;

### Feedback

Lossless conversions require no human supervision. Being lossless, all that really matters is that shit gets smaller, and refract can figure _that_ out on its own. ;)

Lossy conversions are another matter since, by their very nature, information is lost in translation.

For those, refract will present a series of "candidate" images to you, one at a time, in a simple A/B fashion for easy comparison with the original sources.

The "feedback" comes in the form of two buttons — "reject" and "accept" — which can be thought of as answers to the question: Are you happy with this copy?

If it looks like what you want it to, great!, accept it. If not, reject it. Refract will raise or lower the quality of the next candidate accordingly.

Rinse and repeat.

The smallest of the accepted candidates, if any, will be saved to disk at the end of the process, the rest forgotten like a passing dream.

Then it's back around again for the next input/output pair!



## CLI Usage

Refract is a graphical program, but the startup settings and/or queue can be configured via command line if desired.

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
| `--save-auto` | Automatically save successful conversions to their source paths — with new extensions appended — instead of popping a file dialogue for confirmation. |

| Option | Description |
| ------ | ----------- |
| `-l` / `--list` | Read (absolute) image and/or directory paths from this text file, one path per line. Set to "-" to read from STDIN. This is equivalent to specifying the same paths as trailing arguments, but can be cleaner if there are lots of them. |



## Installation

Pre-built packages for x86-64 CPU architectures are available for Debian and Ubuntu users on the [release page](https://github.com/Blobfolio/refract/releases/latest), and to Arch Linux users via [AUR](https://aur.archlinux.org/packages/refract-bin).

To use refract in other environments, it'll needs to be built from source.

Thankfully, [Rust](https://www.rust-lang.org/)/[Cargo](https://github.com/rust-lang/cargo) make this pretty easy:

```bash
# Install the build dependencies. Ubuntu and Debian users, for example,
# can run:
sudo apt-get install -y cmake g++ gcc git make nasm ninja-build

# Build and install refract:
cargo install \
    --git https://github.com/Blobfolio/refract.git \
    --bin refract
```

### Build Dependencies

The extra build dependencies (required by all the damn image codecs) will vary by environment, but at a minimum you'll need up-to-date C and C++ compilers, `cmake`, `git` (obviously), `make`, `nasm`, and `ninja-build`.

Cargo should pop an error if anything's missing. If that happens, just find/install the missing dep and give `cargo install` another shot.

If you wind up needing something not on this list, please [open an issue](https://github.com/Blobfolio/refract/issues) so it can be given a mention. ;)

### Runtime Dependencies

On Linux, the file dialogues require one of `xdg-desktop-portal-[gnome, gtk, kde]` or `zenity`, user's choice.

In theory, _None of the Above_ should work too, provided you use the CLI to enqueue image paths and enable automatic saving with the `--save-auto` flag (along with any other settings tweaks you might want):

```bash
refract --save-auto /path/to/image.jpg
```
