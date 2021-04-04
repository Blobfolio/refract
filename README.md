# Refract

Refract is a guided [WebP](https://en.wikipedia.org/wiki/WebP)/[AVIF](https://en.wikipedia.org/wiki/AV1#AV1_Image_File_Format_(AVIF)) CLI conversion tool for JPEG and PNG image sources. It generates images at various qualities, asking at each step if they look OK, arriving at the smallest acceptable candidate.

It works similarly to the vision tests your eye doctor performs. "How does this look?" Click. "How about now?" Click. "Here's your prescription."

Like your eye doctor, exhaustive A/B comparisons are unnecessary. The quality stepping used by refract pulls the mid-point from moving min/max boundaries. Each answer shrinks the size of the range accordingly, allowing an answer to be discovered in 5-10 steps instead of 100.



## Why?

Every image is different.

You could automate crunching by using a constant quality or performing some form of [SSIM](https://en.wikipedia.org/wiki/Structural_similarity) analysis, but this will result in frequent over- or under-compression.

Obsessive hand-tuning is better, but incredibly tedious.

Refract removes the guesswork and most of the tedium from manual tuning, providing a single command to work against as many files as you wish, with efficient quality iteration and file size tracking. It ensures whatever version is created is the smallest acceptable-looking candidate, and ensures the resulting files do not exceed the size of the original sources (which can happen!).

Should you use this for every image ever? Probably not. But if you have a small web site or home page you're looking to optimize to hell and back, passing the assets through refract is a good idea.



## Installation

This application is written in [Rust](https://www.rust-lang.org/) and can be built using [Cargo](https://github.com/rust-lang/cargo) for Linux or Mac systems.

Pre-built `.deb` packages are also added for each [release](https://github.com/Blobfolio/refract/releases/latest). They should always work for the latest stable Debian and Ubuntu.

Note: WebP and AVIF encoding are handled by the refract binary directly; systems running it do not need `libwebp`, etc., separately installed.



## Usage

It's easy.

Just run `refract [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags are available:

```bash
-h, --help        Prints help information.
    --no-avif     Skip AVIF conversion.
    --no-webp     Skip WebP conversion.
-V, --version     Prints version information.
```

Paths can be any number of individual JPEG or PNG images, or directories containing such images. Paths can also (or additionally) be specified using the `-l`/`--list` option, specifying the path to a text file containing paths one-per-line.

Refract will load each image one-at-a-time and try to generate proposed WebP and/or AVIF copies at varying quality levels. At each step, it will ask you whether or not the proposed image looks good.

(You can view the image in any application of your choosing, though a web browser probably makes the most sense.)

If your answers and the file sizes work out right, a final best-case copy will be created in the source directory with `.webp` or `.avif` appended to the source path, e.g. `/path/to/image.jpg.webp`.

Encoding performance is on par with standalone encoders like `cwebp` and `cavif`. WebP is generally pretty zippy, but AVIF can be a bit slow. If running this against thousands of images, make yourself a pot of coffee ahead of time. :)



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
