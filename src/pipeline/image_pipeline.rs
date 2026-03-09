use std::io::{BufWriter, Write};

use anyhow::Context;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageEncoder, Rgba, RgbaImage};
use log::{debug, info};
use lopdf::{dictionary, Dictionary, Document, Object, Stream};

use crate::cli::args::{CliArgs, Position};
use crate::config::types::WatermarkConfig;
use crate::pipeline::io::{detect_format, resolve_output_path, FileFormat};
use crate::template::TemplateContext;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::qr::generate_qr;
use crate::watermark::create_renderer;
use crate::watermark::filigrane::render_filigrane;

/// Process a single image file (JPEG or PNG), applying the configured watermark.
pub fn process_image(config: &WatermarkConfig, _args: &CliArgs) -> anyhow::Result<()> {
    let input = &config.input;
    let output_path = resolve_output_path(
        input,
        config.output.as_deref(),
        config.suffix.as_deref(),
    );

    if config.dry_run {
        info!(
            "[dry-run] Would watermark {} -> {}",
            input.display(),
            output_path.display()
        );
        return Ok(());
    }

    // 1. Load the source image.
    debug!("Loading image: {}", input.display());
    let source = image::open(input).context("Failed to open input image")?;
    let mut base: RgbaImage = source.to_rgba8();
    let (width, height) = (base.width(), base.height());

    // 2. Create the watermark renderer for the chosen type.
    let renderer = create_renderer(config.watermark_type);

    // 3. Build template context from the input path.
    let _ctx = TemplateContext {
        filename: input
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        ext: input
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        ..Default::default()
    };

    // 4. Render the watermark onto a transparent canvas.
    let mut wm_canvas = renderer
        .render(config, width, height)
        .context("Watermark renderer failed")?;

    // 4b. Overlay cryptographic filigrane security pattern.
    let filigrane = render_filigrane(width, height, config.color, 0.18, config.filigrane);
    wm_canvas.blit(&filigrane, 0, 0);

    // 4c. Overlay QR code if --qr-data was provided.
    if let Some(ref qr_data) = config.qr_data {
        let qr_size = (width.min(height) as f32 * config.scale * 0.5).max(60.0) as u32;
        let color = to_rgba(with_opacity(config.color, config.opacity));
        let qr = generate_qr(qr_data, qr_size, color)
            .context("QR code generation failed")?;
        let qx = (width as i32 - qr.width() as i32) / 2;
        let qy = (height as i32 - qr.height() as i32) / 2;
        wm_canvas.blit(&qr, qx, qy);
    }

    // 4d. Overlay custom image if -I was provided.
    if let Some(ref img_path) = config.image_path {
        overlay_image(&mut wm_canvas, img_path, config)
            .with_context(|| format!("Failed to overlay image: {}", img_path.display()))?;
    }

    // 5. Apply opacity to the watermark canvas.
    let mut wm_image = apply_opacity(wm_canvas.into_image(), config.opacity);

    // 5b. Apply universal perturbation for AI-removal hardening.
    crate::watermark::perturb::perturb(&mut wm_image);

    // 6. Composite the watermark onto the base image.
    composite(&mut base, &wm_image, config);

    // 6b. Apply anti-AI adversarial prompt injection.
    if config.anti_ai {
        crate::watermark::anti_ai::apply_anti_ai(&mut base, config.color);
    }

    // 7. Save the result — use output extension when available, else input.
    let format = detect_format(&output_path).or_else(|_| detect_format(input))?;
    save_image(&base, &output_path, format, config)?;

    info!(
        "Watermarked {} -> {}",
        input.display(),
        output_path.display()
    );
    Ok(())
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Multiply every pixel's alpha channel by `opacity` (0.0 -- 1.0).
fn apply_opacity(mut img: RgbaImage, opacity: f32) -> RgbaImage {
    let factor = opacity.clamp(0.0, 1.0);
    if (factor - 1.0).abs() < f32::EPSILON {
        return img;
    }
    for pixel in img.pixels_mut() {
        pixel.0[3] = (pixel.0[3] as f32 * factor).round() as u8;
    }
    img
}

/// Composite `watermark` onto `base` using the configured position, margin,
/// and offset.  For `Position::Tile` the watermark is repeated across the
/// entire canvas; for `Position::Center` it is placed dead-centre; for the
/// four corner positions it is anchored accordingly.
fn composite(base: &mut RgbaImage, watermark: &RgbaImage, config: &WatermarkConfig) {
    let (bw, bh) = (base.width() as i32, base.height() as i32);
    let (ww, wh) = (watermark.width() as i32, watermark.height() as i32);
    let margin = config.margin as i32;
    let (ox, oy) = config.offset;

    match config.position {
        Position::Tile => {
            let spacing = config.tile_spacing as i32;
            let step_x = ww + spacing;
            let step_y = wh + spacing;
            if step_x <= 0 || step_y <= 0 {
                return;
            }
            let mut y = 0;
            while y < bh {
                let mut x = 0;
                while x < bw {
                    alpha_blend(base, watermark, x, y);
                    x += step_x;
                }
                y += step_y;
            }
        }
        _ => {
            let (x, y) = anchor_position(bw, bh, ww, wh, config.position, margin);
            alpha_blend(base, watermark, x + ox, y + oy);
        }
    }
}

/// Return the top-left `(x, y)` coordinates for placing a rectangle of size
/// `(ww, wh)` inside a canvas of size `(bw, bh)` at the given anchor with the
/// specified margin.
fn anchor_position(bw: i32, bh: i32, ww: i32, wh: i32, pos: Position, margin: i32) -> (i32, i32) {
    match pos {
        Position::Center => ((bw - ww) / 2, (bh - wh) / 2),
        Position::TopLeft => (margin, margin),
        Position::TopRight => (bw - ww - margin, margin),
        Position::BottomLeft => (margin, bh - wh - margin),
        Position::BottomRight => (bw - ww - margin, bh - wh - margin),
        Position::Tile => (0, 0), // handled separately
    }
}

/// Alpha-blend `overlay` onto `base` with its top-left corner at `(dx, dy)`.
fn alpha_blend(base: &mut RgbaImage, overlay: &RgbaImage, dx: i32, dy: i32) {
    let (bw, bh) = (base.width() as i32, base.height() as i32);
    let (ow, oh) = (overlay.width() as i32, overlay.height() as i32);

    let x_start = dx.max(0);
    let y_start = dy.max(0);
    let x_end = (dx + ow).min(bw);
    let y_end = (dy + oh).min(bh);

    for y in y_start..y_end {
        for x in x_start..x_end {
            let sx = (x - dx) as u32;
            let sy = (y - dy) as u32;
            let fg = overlay.get_pixel(sx, sy);
            let alpha = fg.0[3] as f32 / 255.0;
            if alpha <= 0.0 {
                continue;
            }
            let bg = base.get_pixel(x as u32, y as u32);
            let inv = 1.0 - alpha;
            let blended = Rgba([
                (fg.0[0] as f32 * alpha + bg.0[0] as f32 * inv).round() as u8,
                (fg.0[1] as f32 * alpha + bg.0[1] as f32 * inv).round() as u8,
                (fg.0[2] as f32 * alpha + bg.0[2] as f32 * inv).round() as u8,
                (bg.0[3] as f32 + fg.0[3] as f32 * inv).min(255.0).round() as u8,
            ]);
            base.put_pixel(x as u32, y as u32, blended);
        }
    }
}

/// Save an RGBA image to `path` respecting format-specific quality settings.
fn save_image(
    img: &RgbaImage,
    path: &std::path::Path,
    format: FileFormat,
    config: &WatermarkConfig,
) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("Failed to create output file: {}", path.display()))?;
    let writer = BufWriter::new(file);

    match format {
        FileFormat::Jpeg => {
            // JPEG does not support alpha – convert to RGB first.
            let rgb = DynamicImage::ImageRgba8(img.clone()).to_rgb8();
            let encoder = JpegEncoder::new_with_quality(writer, config.quality);
            encoder.write_image(
                &rgb,
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )?;
        }
        FileFormat::Png => {
            let compression = match config.png_compression {
                0 => CompressionType::Fast,
                1..=5 => CompressionType::Default,
                _ => CompressionType::Best,
            };
            let encoder = PngEncoder::new_with_quality(writer, compression, FilterType::Adaptive);
            encoder.write_image(
                img,
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )?;
        }
        FileFormat::Pdf => {
            drop(writer); // close the empty file — lopdf writes its own way
            save_image_as_pdf(img, path)?;
        }
    }

    Ok(())
}

/// Embed a watermarked RGBA image into a single-page PDF.
fn save_image_as_pdf(img: &RgbaImage, path: &std::path::Path) -> anyhow::Result<()> {
    let w = img.width();
    let h = img.height();

    // Split RGBA → RGB + alpha, compress both.
    let px_count = (w * h) as usize;
    let mut rgb = Vec::with_capacity(px_count * 3);
    let mut alpha = Vec::with_capacity(px_count);
    for px in img.pixels() {
        rgb.push(px[0]);
        rgb.push(px[1]);
        rgb.push(px[2]);
        alpha.push(px[3]);
    }

    let rgb_z = deflate(&rgb);
    let alpha_z = deflate(&alpha);

    let mut doc = Document::with_version("1.7");

    // SMask (alpha channel).
    let smask_dict = dictionary! {
        "Type" => Object::Name(b"XObject".to_vec()),
        "Subtype" => Object::Name(b"Image".to_vec()),
        "Width" => Object::Integer(w as i64),
        "Height" => Object::Integer(h as i64),
        "ColorSpace" => Object::Name(b"DeviceGray".to_vec()),
        "BitsPerComponent" => Object::Integer(8),
        "Filter" => Object::Name(b"FlateDecode".to_vec()),
        "Length" => Object::Integer(alpha_z.len() as i64),
    };
    let smask_id = doc.add_object(Stream::new(smask_dict, alpha_z));

    // Image XObject (RGB + SMask).
    let mut img_dict = dictionary! {
        "Type" => Object::Name(b"XObject".to_vec()),
        "Subtype" => Object::Name(b"Image".to_vec()),
        "Width" => Object::Integer(w as i64),
        "Height" => Object::Integer(h as i64),
        "ColorSpace" => Object::Name(b"DeviceRGB".to_vec()),
        "BitsPerComponent" => Object::Integer(8),
        "Filter" => Object::Name(b"FlateDecode".to_vec()),
        "Length" => Object::Integer(rgb_z.len() as i64),
    };
    img_dict.set("SMask", Object::Reference(smask_id));
    let img_id = doc.add_object(Stream::new(img_dict, rgb_z));

    // Page content: draw image scaled to full page.
    let content = format!("q\n{w} 0 0 {h} 0 0 cm\n/Img Do\nQ\n");
    let content_id = doc.add_object(Stream::new(Dictionary::new(), content.into_bytes()));

    // Resources.
    let mut xobjects = Dictionary::new();
    xobjects.set("Img", Object::Reference(img_id));
    let resources = dictionary! {
        "XObject" => xobjects,
    };
    let resources_id = doc.add_object(resources);

    // Page.
    let page = dictionary! {
        "Type" => Object::Name(b"Page".to_vec()),
        "MediaBox" => vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Integer(w as i64),
            Object::Integer(h as i64),
        ],
        "Contents" => Object::Reference(content_id),
        "Resources" => Object::Reference(resources_id),
    };
    let page_id = doc.add_object(page);

    // Pages tree.
    let pages = dictionary! {
        "Type" => Object::Name(b"Pages".to_vec()),
        "Kids" => vec![Object::Reference(page_id)],
        "Count" => Object::Integer(1),
    };
    let pages_id = doc.add_object(pages);

    // Back-link Parent.
    if let Ok(page_obj) = doc.get_object_mut(page_id) {
        if let Ok(d) = page_obj.as_dict_mut() {
            d.set("Parent", Object::Reference(pages_id));
        }
    }

    // Catalog.
    let catalog = dictionary! {
        "Type" => Object::Name(b"Catalog".to_vec()),
        "Pages" => Object::Reference(pages_id),
    };
    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    doc.save(path)
        .with_context(|| format!("Failed to save PDF: {}", path.display()))?;
    Ok(())
}

fn deflate(data: &[u8]) -> Vec<u8> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::default());
    enc.write_all(data).expect("deflate write");
    enc.finish().expect("deflate finish")
}

/// Load an external image, scale it to fit within the watermark canvas, and
/// blit it centred.  The image respects `config.scale` and `config.opacity`.
fn overlay_image(
    canvas: &mut crate::render::canvas::Canvas,
    path: &std::path::Path,
    config: &WatermarkConfig,
) -> anyhow::Result<()> {
    use crate::render::canvas::Canvas as C;
    use crate::render::transform::scale_canvas;

    let src = image::open(path)
        .with_context(|| format!("Failed to open overlay image: {}", path.display()))?;
    let rgba = src.to_rgba8();
    let src_canvas = C::from_image(rgba);

    // Scale the overlay so its longest side is `config.scale` of the canvas.
    let target = (canvas.width().min(canvas.height()) as f32 * config.scale).max(32.0);
    let longest = src_canvas.width().max(src_canvas.height()) as f32;
    let factor = target / longest;
    let scaled = if (factor - 1.0).abs() > 0.01 {
        scale_canvas(&src_canvas, factor)
    } else {
        src_canvas
    };

    // Apply watermark color opacity to the overlay pixels.
    let opacity = config.opacity.clamp(0.0, 1.0);
    let mut tinted = C::new(scaled.width(), scaled.height());
    for y in 0..scaled.height() {
        for x in 0..scaled.width() {
            let mut px = *scaled.image().get_pixel(x, y);
            px[3] = (px[3] as f32 * opacity).round() as u8;
            if px[3] > 0 {
                tinted.set_pixel(x as i32, y as i32, px);
            }
        }
    }

    let ox = (canvas.width() as i32 - tinted.width() as i32) / 2;
    let oy = (canvas.height() as i32 - tinted.height() as i32) / 2;
    canvas.blit(&tinted, ox, oy);

    Ok(())
}
