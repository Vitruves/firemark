use std::io::{BufWriter, Write};

use anyhow::Context;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::{DynamicImage, ImageEncoder, Rgba, RgbaImage};
use log::{debug, info};
use lopdf::{dictionary, Dictionary, Document, Object, Stream};

use rand::seq::SliceRandom;
use rand::Rng;

use crate::cli::args::{BlendMode, CliArgs, Position};
use crate::config::types::WatermarkConfig;
use crate::pipeline::io::{detect_format, resolve_output_path, FileFormat};
use crate::template::TemplateContext;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::compositor::{get_blend_fn, BlendFn};
use crate::render::qr::generate_qr;
use crate::render::saliency::SaliencyMap;
use crate::render::transform::{rotate_canvas, scale_canvas};
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
    let fil_opacity = config.opacity * 0.36; // scales with --opacity (0.18 at default 0.5)
    let filigrane = render_filigrane(width, height, config.color, fil_opacity, config.filigrane);
    wm_canvas.blit(&filigrane, 0, 0);

    // 4c. Overlay QR code if --qr-data was provided.
    if let Some(ref qr_data) = config.qr_data {
        let qr_size = config.qr_code_size
            .unwrap_or_else(|| (width.min(height) as f32 * config.scale * 0.5).max(60.0) as u32);
        let color = to_rgba(with_opacity(config.color, config.opacity));
        let qr = generate_qr(qr_data, qr_size, color)
            .context("QR code generation failed")?;
        let (qx, qy) = qr_position(width, height, qr.width(), qr.height(), config.qr_code_position, config.margin);
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

    // 6b. Apply stroke entanglement + anti-AI adversarial prompt injection.
    if config.anti_ai {
        crate::watermark::entangle::entangle_strokes(&mut base, config.color, config.opacity);
        crate::watermark::anti_ai::apply_anti_ai(&mut base, config.color, config.opacity);
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

// ── Public helpers ──────────────────────────────────────────────────────────

/// Compute the top-left (x, y) for placing a QR code of size (qw, qh) inside
/// a canvas of size (cw, ch) at the given position with the specified margin.
pub fn qr_position(cw: u32, ch: u32, qw: u32, qh: u32, pos: Position, margin: u32) -> (i32, i32) {
    let (cw, ch, qw, qh, m) = (cw as i32, ch as i32, qw as i32, qh as i32, margin as i32);
    match pos {
        Position::Center => ((cw - qw) / 2, (ch - qh) / 2),
        Position::TopLeft => (m, m),
        Position::TopRight => (cw - qw - m, m),
        Position::BottomLeft => (m, ch - qh - m),
        Position::BottomRight => (cw - qw - m, ch - qh - m),
        Position::Tile => ((cw - qw) / 2, (ch - qh) / 2), // tile makes no sense for QR; fall back to center
    }
}

// ── Internal helpers ────────────────────────────────────────────────────────

/// Multiply every pixel's alpha channel by `opacity` (0.0 -- 1.0).
pub(crate) fn apply_opacity(mut img: RgbaImage, opacity: f32) -> RgbaImage {
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
///
/// All compositing goes through the entangled blender (see `blend_entangled`)
/// and tile mode uses per-tile jitter, rotation, scale, and opacity variation
/// with saliency-biased placement, so removal generalizes poorly: solving one
/// tile does not solve the others, and inpainting must touch real content.
fn composite(base: &mut RgbaImage, watermark: &RgbaImage, config: &WatermarkConfig) {
    let (bw, bh) = (base.width() as i32, base.height() as i32);
    let (ww, wh) = (watermark.width() as i32, watermark.height() as i32);
    let margin = config.margin as i32;
    let (ox, oy) = config.offset;

    let mut rng = rand::thread_rng();
    let fields = ModulationFields::new(base.width(), base.height(), &mut rng);
    // Plain alpha-over carries no dependence on the underlying pixels, so by
    // default (None) the entangled color component uses luminance-adaptive
    // multiply/screen, which keeps the mark at full strength on flat white or
    // black while entangling it with midtone content.
    let blend = match config.blend {
        BlendMode::Normal => None,
        m => Some(get_blend_fn(m)),
    };

    match config.position {
        // Full-canvas marks (the full-page renderers) have nothing to tile.
        Position::Tile if ww >= bw && wh >= bh => {
            blend_entangled(base, watermark, 0, 0, 1.0, blend, &fields);
        }
        Position::Tile => {
            let spacing = config.tile_spacing as i32;
            let step_x = ww + spacing;
            let step_y = wh + spacing;
            if step_x <= 0 || step_y <= 0 {
                return;
            }

            let saliency = SaliencyMap::from_image(base);
            let variants = tile_variants(watermark, &mut rng);
            let jitter = (spacing / 2).max(step_x.min(step_y) / 8).max(4);

            let mut y = 0;
            while y < bh {
                let mut x = 0;
                while x < bw {
                    let variant = variants.choose(&mut rng).expect("variants non-empty");
                    let (vw, vh) = (variant.width(), variant.height());
                    // Sample a few jittered candidates and keep the one that
                    // overlaps the most document content.
                    let mut best = (x, y);
                    let mut best_score = -1.0f32;
                    for _ in 0..3 {
                        let cx = x + rng.gen_range(-jitter..=jitter);
                        let cy = y + rng.gen_range(-jitter..=jitter);
                        let score = saliency.region_score(cx, cy, vw, vh);
                        if score > best_score {
                            best_score = score;
                            best = (cx, cy);
                        }
                    }
                    let alpha_factor = rng.gen_range(0.75..1.1);
                    blend_entangled(base, variant, best.0, best.1, alpha_factor, blend, &fields);
                    x += step_x;
                }
                y += step_y;
            }
        }
        _ => {
            let (x, y) = anchor_position(bw, bh, ww, wh, config.position, margin);
            blend_entangled(base, watermark, x + ox, y + oy, 1.0, blend, &fields);
        }
    }
}

/// Pre-render a handful of rotated/scaled variants of the tile watermark so
/// no two tiles are pixel-identical and a removal model cannot solve one tile
/// and apply the solution everywhere.
fn tile_variants(wm: &RgbaImage, rng: &mut impl Rng) -> Vec<RgbaImage> {
    let mut out = Vec::with_capacity(4);
    // Padding absorbs corner clipping for rotations up to ~14° (sin 14° ≈ 0.24).
    let pad = (wm.width().max(wm.height()) as f32 * 0.13) as u32 + 2;
    for _ in 0..4 {
        let angle = rng.gen_range(-14.0f32..14.0);
        let scale = rng.gen_range(0.85f32..1.15);
        let mut padded = Canvas::new(wm.width() + pad * 2, wm.height() + pad * 2);
        padded.blit(&Canvas::from_image(wm.clone()), pad as i32, pad as i32);
        let rotated = rotate_canvas(&padded, angle);
        out.push(scale_canvas(&rotated, scale).into_image());
    }
    out
}

/// Low-frequency modulation fields — sums of randomly-phased sines whose
/// wavelengths span a third to a full canvas dimension. They vary the mark's
/// color mix and opacity smoothly across the image so no constant opacity or
/// hue signature exists for a segmentation model to key on, while staying
/// gradual enough to be invisible to a human reader.
struct ModulationFields {
    color: [(f32, f32, f32); 2],
    alpha: [(f32, f32, f32); 2],
}

impl ModulationFields {
    fn new(w: u32, h: u32, rng: &mut impl Rng) -> Self {
        let dim = w.min(h).max(1) as f32;
        let field = |rng: &mut dyn rand::RngCore| {
            let wavelength = dim * rng.gen_range(0.3..1.0);
            let dir: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
            let f = std::f32::consts::TAU / wavelength;
            (f * dir.cos(), f * dir.sin(), rng.gen_range(0.0..std::f32::consts::TAU))
        };
        Self {
            color: [field(rng), field(rng)],
            alpha: [field(rng), field(rng)],
        }
    }

    fn eval(pair: &[(f32, f32, f32); 2], x: f32, y: f32) -> f32 {
        let a = (pair[0].0 * x + pair[0].1 * y + pair[0].2).sin();
        let b = (pair[1].0 * x + pair[1].1 * y + pair[1].2).sin();
        0.5 + 0.25 * a + 0.25 * b
    }

    /// How much pure mark color vs. background-entangled color, in [0, 1].
    fn color_mix(&self, x: f32, y: f32) -> f32 {
        Self::eval(&self.color, x, y)
    }

    /// Opacity modulation, in [0, 1].
    fn alpha_mod(&self, x: f32, y: f32) -> f32 {
        Self::eval(&self.alpha, x, y)
    }
}

/// Composite `overlay` onto `base` at `(dx, dy)`, entangling the mark with
/// the underlying content: each pixel's color is a spatially-varying mix of
/// the mark color and a blend with the background (`None` = adaptive
/// multiply/screen chosen per pixel by background luminance), and its opacity
/// is modulated by a low-frequency field and `alpha_factor`.
fn blend_entangled(
    base: &mut RgbaImage,
    overlay: &RgbaImage,
    dx: i32,
    dy: i32,
    alpha_factor: f32,
    blend: Option<BlendFn>,
    fields: &ModulationFields,
) {
    let multiply = get_blend_fn(BlendMode::Multiply);
    let screen = get_blend_fn(BlendMode::Screen);
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
            if fg.0[3] == 0 {
                continue;
            }

            let (fx, fy) = (x as f32, y as f32);
            let amod = 0.72 + 0.56 * fields.alpha_mod(fx, fy);
            let alpha = (fg.0[3] as f32 / 255.0 * alpha_factor * amod).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }

            let bg = base.get_pixel(x as u32, y as u32);
            let bg_lum = crate::render::color::luminance(bg);
            let entangle_fn = blend.unwrap_or(if bg_lum >= 128.0 { multiply } else { screen });
            let mix_t = 0.35 + 0.4 * fields.color_mix(fx, fy);
            let inv = 1.0 - alpha;
            let mut out = [0u8; 4];
            for (ch, o) in out.iter_mut().take(3).enumerate() {
                let entangled = entangle_fn(bg.0[ch], fg.0[ch]) as f32;
                let ink = fg.0[ch] as f32 * mix_t + entangled * (1.0 - mix_t);
                *o = (ink * alpha + bg.0[ch] as f32 * inv).round().clamp(0.0, 255.0) as u8;
            }
            out[3] = (bg.0[3] as f32 + fg.0[3] as f32 * inv).min(255.0).round() as u8;
            base.put_pixel(x as u32, y as u32, Rgba(out));
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
        FileFormat::WebP => {
            let encoder = image::codecs::webp::WebPEncoder::new_lossless(writer);
            encoder.write_image(
                img,
                img.width(),
                img.height(),
                image::ExtendedColorType::Rgba8,
            )?;
        }
        FileFormat::Tiff => {
            let encoder = image::codecs::tiff::TiffEncoder::new(writer);
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
