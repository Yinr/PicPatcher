# PicPatcher

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)

[English](./README.md) | [中文](./README_zh.md)

Batch image overlay tool. Stamp a fixed overlay image (e.g. a date watermark)
at a fixed pixel position on every matching image in a directory.

Two binaries from one Rust workspace:

- `picpatcher` (CLI) — batch runner driven by a JSON config file.
- `picpatcher-gui` (GUI) — visual editor (load base + overlay, drag to place,
  export config) and integrated runner.

## Build

```
cargo build --release
```

Binaries land in `target/release/`.
On Windows, both `picpatcher-gui.exe` and `picpatcher.exe` embed `assets/icon.ico` as their file icon.

## CLI usage

```
picpatcher --config picpatcher.json --input ./photos --recursive
```

Flags:

- `--config <FILE>` JSON config (produced by the GUI).
- `--input <DIR>` directory to scan.
- `--ext jpg,png,…` optional extension filter
  (default: `jpg,jpeg,png,webp,bmp`).
- `--recursive` recurse into subdirectories.
- `--in-place` overwrite originals (destructive).
- `--out-dir <NAME>` sibling output dir name when not in-place
  (default: `_modified`).
- `--dry-run` list planned input/output paths without writing anything.

## Config format (v1)

```json
{
  "version": 1,
  "overlay": "./stamps/date.png",
  "x": 1200,
  "y": 80
}
```

`overlay` may be absolute or relative to the config file's directory.

## GUI

Run `picpatcher-gui`. Two tabs:

- **Editor** — open base image + overlay; zoom the canvas for detail work;
  drag overlay on the canvas; nudge with arrow keys (`Shift` = 10 px);
  edit X/Y; Save / Save as config.
- **Runner** — pick config and input dir; Run; live progress and error log.

## Notes

- JPEG output uses quality 95.
- Overlay with alpha is blended (Porter–Duff `over`).
- First release supports a single overlay region. Multi-region, scale/rotate,
  EXIF preservation are deliberately out of scope for v1.

## Roadmap

Planned features and ideas for future versions live in [ROADMAP.md](./ROADMAP.md).

## License

MIT. See [LICENSE](./LICENSE).
