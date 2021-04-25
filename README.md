# Refract

**Note:** Chrome v.91 is introducing stricter requirements for AVIF images that will [prevent the rendering of many previously valid sources](https://bugs.chromium.org/p/chromium/issues/detail?id=1115483), including those created by GIMP, cavif, refract, libheif, image.rs, sharp, and others. For the time being, it is recommended you disable AVIF generation in this program by passing the `--no-avif` flag.

Refract is a guided CLI image conversion tool. It takes [JPEG](https://en.wikipedia.org/wiki/JPEG) and [PNG](https://en.wikipedia.org/wiki/Portable_Network_Graphics) image sources and produces [AVIF](https://en.wikipedia.org/wiki/AV1#AV1_Image_File_Format_(AVIF)), [JPEG XL](https://en.wikipedia.org/wiki/JPEG_XL), and [WebP](https://en.wikipedia.org/wiki/WebP) clones.

The program is named after — and works like — eye doctor Refraction Tests. It generates images at various qualities, asking at each step how it looks, until it arrives at the smallest acceptable candidate possible.

![Example CLI output.](https://github.com/Blobfolio/refract/raw/master/skel/prompt.png)

The beauty of this approach is that it moots the need for exhaustive testing. Refract's quality stepping works by selecting the mid-point between moving min/max boundaries. Each answer you provide adjusts the range, allowing the final, perfect result to be discovered in just 5-10 steps instead of 100+.



## Why?

Every image is different. The idea of a "Magic Bullet" format is a myth.

If you want to truly maximize the quality and size of next-gen copies, you cannot rely on hardcoded constants or fancy [SSIM](https://en.wikipedia.org/wiki/Structural_similarity) analysis. That will result in frequent over- or under-compression, and some images will come out looking… bad.

You have to actually use your eyes. And you have to pay attention to the resulting file sizes. Sometimes newer formats fall short of what the originals achieved.

While you can do all of this manually, running multiple programs hundreds of times for each and every source you want to convert, it is easy to screw up, and really tedious.

Refract helps makes that manual process _less_ tedious.

It automatically uses the strongest (slowest) possible compression settings for each format, keeps track of file sizes and qualities, can process inputs en masse, and reduces the number of conversion tests by around 90%.

Should you use it for every image ever? No, probably not. The next generation formats, particularly AVIF and JPEG XL, require a lot of computation to discover byte savings. All those minutes will add up quickly.

But if you're looking to obsessively optimize a small project or single web page, Refract is the way to go!



## Features

Only JPEG and PNG input sources are supported. They can have RGB, RGBA, greyscale, or greyscale w/ alpha color spaces, but CMYK is not supported.

Conversion is done at pixel level; gamma and other metadata profile information is not factored or present in the converted copies, so is not supported.

The following compression modes are attempted when generating next-gen image candidates. The under-the-hood implementations are a bit different, but performance and savings are comparable to the referenced third-party commands:

| Encoding | Mode | Inputs | Comparable To Running |
| -------- | ---- | ---- | ---- |
| AVIF | Lossy | JPEG, PNG | `cavif --speed 1 --quality <N>` |
| JPEG XL | Lossless | JPEG, PNG | `cjxl --speed tortoise --distance 0.0` |
| JPEG XL | Lossy | JPEG, PNG | `cjxl --speed tortoise --distance <N>` |
| WebP | Lossless | PNG | `cwebp -lossless -z 9 -q 100` |
| WebP | Lossy | JPEG, PNG | `cwebp -m 6 -pass 10 -q <N>` |

Both AVIF and JPEG XL are CPU-aware and will leverage thread parallelization when encoding. This provides massive, but relative, performance improvements for these famously _slow_ formats.

Speaking of AVIF and JPEG XL, both standards are still cutting edge, and will likely see a number of upstream improvements going forward. They'll probably never be anywhere near as fast as WebP (which will never be anywhere near as fast as JPEG), but should get faster than they are today.



## Usage

It's easy.

Just run `refract [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags are available:

```bash
-h, --help        Prints help information.
    --no-avif     Skip AVIF conversion.
    --no-jxl      Skip JPEG XL conversion.
    --no-webp     Skip WebP conversion.
-V, --version     Prints version information.
```

By default, Refract will generate copies in every next-gen format. If you want to skip one, use the corresponding `--no-*` flag.

You can specify any number of input paths. The paths can be individual JPEG or PNG images, or directories containing such images. You can also/alternatively specify paths using the `-l`/`--list` option, which should point to a text file containing any number of paths, one per line.

```bash
# Handle one image.
refract /path/to/image.jpg

# Example pulling paths from a text file.
refract --list /path/to/list.txt /path/to/another/image.jpg

# Skip WebP.
refract --no-webp /path/to/image.jpg
```

Refract will load each input image one-at-a-time and try to generate proposed AVIF, JPEG XL, and/or WebP copies at varying quality levels. At each step, it will ask you whether or not the proposed image looks good.

You can preview the candidate images in any program of your choice. Whatever you're comfortable with. We like using web browsers for this.

If your answers and the file sizes work out right, a final best-case copy of each image will be created in the source directory with `.avif`, `.jxl`, or `.webp` appended to the source path, e.g. `/path/to/image.jpg.webp`.



## Installation

Refract is a Linux-first application. Pre-built `.deb` packages are added to each [release](https://github.com/Blobfolio/refract/releases/latest) for Debian and Ubuntu users. (These packages could also be installed in a Debian/Ubuntu Docker container and run on any host.)

But it is written in [Rust](https://www.rust-lang.org/) and so can be built from source using [Cargo](https://github.com/rust-lang/cargo) on other x86-64 Unix platforms (Mac, etc.).

Note that if building from source, you'll also need NASM, GCC, and Clang installed.



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
