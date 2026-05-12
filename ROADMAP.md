# Roadmap

Deferred features and ideas captured during initial design. Items are roughly
ordered by expected value, not by implementation order.

## v1.x — incremental polish

- [ ] **JPEG quality flag** — expose `--jpeg-quality <1-100>` on the CLI and a
      matching field in the config (currently hard-coded to 95).
- [ ] **Parallelism control** — wire `rayon` thread pool to a `-j / --jobs` flag.
- [ ] **Progress bar** — `indicatif` progress in the CLI for large batches.
- [ ] **Dry-run report details** — `--dry-run` lists planned output paths; also
      emit a summary (count by extension, skipped files, would-overwrite warnings).
- [ ] **GUI: "Preview to temp dir"** — one-click button that runs the current
      unsaved config against the input dir into a throwaway folder and opens it.
- [ ] **Config validation command** — `picpatcher check --config foo.json`
      verifies overlay path exists and coordinates are non-negative.

## v2 — multi-region & relative anchors

- [ ] **Multiple regions per config** — bump config to `version: 2` with
      `regions: [{ overlay, x, y, anchor, blend }]`. Keep v1 loader for
      backward compatibility.
- [ ] **Anchor points** — `top-left | top-right | bottom-left | bottom-right |
      center`, so a single config works across mixed resolutions.
- [ ] **Blend modes** — `over` (alpha composite, current) vs `replace`
      (hard pixel copy, ignores overlay alpha).
- [ ] **GUI multi-region editor** — list panel, per-region selection,
      add/remove, drag any region independently.

## v3 — transforms & metadata

- [ ] **Overlay scale / rotate** — per-region `scale` and `rotation_deg`.
- [ ] **EXIF preservation** — read with `kamadak-exif`, re-attach after
      re-encode so JPGs keep capture time, camera info, orientation.
- [ ] **More formats** — verify and document WebP, TIFF, GIF (first frame).
- [ ] **Color-managed JPEG** — preserve ICC profile when present.

## Nice-to-have / exploratory

- [ ] **Undo / redo** in the GUI editor.
- [ ] **Snap to grid / guides** when dragging.
- [ ] **Per-file overrides** — glob-based rules in the config to use different
      overlays for different filename patterns.
- [ ] **Watch mode** — `picpatcher watch --input ./dropbox` processes files as
      they appear.
- [ ] **Headless library crate** — expose `picpatcher-core` on crates.io so
      others can embed the compose logic.
- [ ] **Cross-platform packaging** — pre-built binaries for Windows / macOS /
      Linux via GitHub Actions.

## Explicitly out of scope

- General-purpose image editor features (filters, layers, text tools).
- Cloud sync / remote storage backends.
- OCR-driven dynamic overlay placement.
