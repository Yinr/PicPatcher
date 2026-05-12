use std::path::Path;

use anyhow::{Context, Result};
use image::{DynamicImage, ImageFormat, RgbaImage};

/// Load an overlay image, decoded to RGBA8 for alpha-aware compositing.
pub fn load_overlay(path: &Path) -> Result<RgbaImage> {
    let img =
        image::open(path).with_context(|| format!("failed to open overlay: {}", path.display()))?;
    Ok(img.to_rgba8())
}

/// Paste `overlay` on top of `base` at (x, y) using alpha blending.
/// Pixels of overlay outside `base` are clipped.
pub fn apply_to_image(base: &mut DynamicImage, overlay: &RgbaImage, x: i64, y: i64) {
    // Convert base to RGBA8 in-place if necessary by replacing.
    if !matches!(base, DynamicImage::ImageRgba8(_)) {
        *base = DynamicImage::ImageRgba8(base.to_rgba8());
    }
    let base_rgba = match base {
        DynamicImage::ImageRgba8(b) => b,
        _ => unreachable!(),
    };

    let (bw, bh) = (base_rgba.width() as i64, base_rgba.height() as i64);
    let (ow, oh) = (overlay.width() as i64, overlay.height() as i64);

    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + ow).min(bw);
    let y1 = (y + oh).min(bh);
    if x0 >= x1 || y0 >= y1 {
        return;
    }

    for py in y0..y1 {
        for px in x0..x1 {
            let op = overlay.get_pixel((px - x) as u32, (py - y) as u32);
            let oa = op.0[3] as u32;
            if oa == 0 {
                continue;
            }
            if oa == 255 {
                base_rgba.put_pixel(px as u32, py as u32, *op);
                continue;
            }
            let bp = base_rgba.get_pixel(px as u32, py as u32);
            let inv = 255 - oa;
            let mix = |o: u8, b: u8| -> u8 { ((o as u32 * oa + b as u32 * inv) / 255) as u8 };
            let blended = image::Rgba([
                mix(op.0[0], bp.0[0]),
                mix(op.0[1], bp.0[1]),
                mix(op.0[2], bp.0[2]),
                (oa + bp.0[3] as u32 * inv / 255).min(255) as u8,
            ]);
            base_rgba.put_pixel(px as u32, py as u32, blended);
        }
    }
}

/// Save image, picking a sensible encoder. JPEG uses quality 95.
pub fn save_image(img: &DynamicImage, path: &Path) -> Result<()> {
    let format = ImageFormat::from_path(path)
        .with_context(|| format!("cannot determine format for: {}", path.display()))?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("failed to create output directory: {}", parent.display())
            })?;
        }
    }
    match format {
        ImageFormat::Jpeg => {
            // JPEG has no alpha; convert to RGB8 to avoid encoder error.
            let rgb = img.to_rgb8();
            let file = std::fs::File::create(path)
                .with_context(|| format!("failed to create: {}", path.display()))?;
            let writer = std::io::BufWriter::new(file);
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(writer, 95);
            encoder
                .encode(
                    &rgb,
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .with_context(|| format!("failed to encode JPEG: {}", path.display()))?;
        }
        _ => {
            img.save_with_format(path, format)
                .with_context(|| format!("failed to save: {}", path.display()))?;
        }
    }
    Ok(())
}

/// Convenience: open, apply, save.
pub fn process_file(
    input: &Path,
    output: &Path,
    overlay: &RgbaImage,
    x: i64,
    y: i64,
) -> Result<()> {
    let mut img =
        image::open(input).with_context(|| format!("failed to open image: {}", input.display()))?;
    apply_to_image(&mut img, overlay, x, y);
    save_image(&img, output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba};

    #[test]
    fn apply_to_image_replaces_opaque_overlay_pixels() {
        let mut base = DynamicImage::ImageRgba8(RgbaImage::from_pixel(4, 4, Rgba([0, 0, 0, 255])));
        let overlay = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));

        apply_to_image(&mut base, &overlay, 1, 1);
        let out = base.to_rgba8();

        assert_eq!(*out.get_pixel(0, 0), Rgba([0, 0, 0, 255]));
        assert_eq!(*out.get_pixel(1, 1), Rgba([255, 0, 0, 255]));
        assert_eq!(*out.get_pixel(2, 2), Rgba([255, 0, 0, 255]));
    }

    #[test]
    fn apply_to_image_clips_overlay_outside_bounds() {
        let mut base = DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 2, Rgba([0, 0, 0, 255])));
        let overlay = RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255]));

        apply_to_image(&mut base, &overlay, -1, -1);
        let out = base.to_rgba8();

        assert_eq!(*out.get_pixel(0, 0), Rgba([0, 255, 0, 255]));
        assert_eq!(*out.get_pixel(1, 1), Rgba([0, 0, 0, 255]));
    }
}
