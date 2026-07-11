use std::collections::BTreeMap;
use std::io::Write;

use anyhow::Context;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use log::{debug, info, warn};
use lopdf::{Dictionary, Document, Object, Stream};

use crate::cli::args::CliArgs;
use crate::config::types::WatermarkConfig;
use crate::pipeline::io::resolve_output_path;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::qr::generate_qr;
use crate::watermark;
use crate::watermark::filigrane::render_filigrane;

/// Process a single PDF file — render watermark to canvas, embed as image overlay.
///
/// This uses the exact same renderer pipeline as image watermarking, producing
/// identical results.  The rendered canvas is embedded as a transparent PNG
/// image XObject on each page.
pub fn process_pdf(config: &WatermarkConfig, args: &CliArgs) -> anyhow::Result<()> {
    let input = &config.input;
    let output_path =
        resolve_output_path(input, config.output.as_deref(), config.suffix.as_deref());

    // If output format is an image, rasterize the PDF and use the image pipeline.
    match crate::pipeline::io::detect_format(&output_path) {
        Ok(crate::pipeline::io::FileFormat::Pdf) => {}
        Ok(_) => {
            return process_pdf_to_image(config, args, &output_path);
        }
        Err(_) => {}
    }

    if config.dry_run {
        info!(
            "[dry-run] Would watermark PDF {} -> {}",
            input.display(),
            output_path.display()
        );
        return Ok(());
    }

    debug!("Loading PDF: {}", input.display());
    let mut doc = Document::load(input)
        .with_context(|| format!("Failed to load PDF: {}", input.display()))?;

    let page_range = &config.pages;
    let page_ids: Vec<(u32, lopdf::ObjectId)> = doc
        .get_pages()
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .into_iter()
        .collect();

    let renderer = watermark::create_renderer(config.watermark_type);

    for (page_num, page_id) in &page_ids {
        if !page_range.contains(*page_num) {
            continue;
        }
        if let Some(ref skip) = config.skip_pages {
            if skip.contains(*page_num) {
                debug!("Skipping page {page_num} (in skip list)");
                continue;
            }
        }

        let (page_width_pt, page_height_pt) = get_page_dimensions(&doc, *page_id)?;

        // Render at the configured DPI. PDF points = 1/72 inch.
        let dpi = config.dpi.max(72) as f32;
        let px_w = (page_width_pt * dpi / 72.0).round() as u32;
        let px_h = (page_height_pt * dpi / 72.0).round() as u32;

        let mut canvas = renderer
            .render(config, px_w, px_h)
            .with_context(|| format!("Failed to render watermark for page {page_num}"))?;

        // Overlay cryptographic filigrane security pattern.
        let fil_opacity = config.opacity * 0.36; // scales with --opacity (0.18 at default 0.5)
        let filigrane = render_filigrane(px_w, px_h, config.color, fil_opacity, config.filigrane);
        canvas.blit(&filigrane, 0, 0);

        // Overlay QR code if --qr-data was provided.
        if let Some(ref qr_data) = config.qr_data {
            let qr_size = config
                .qr_code_size
                .unwrap_or_else(|| (px_w.min(px_h) as f32 * config.scale * 0.5).max(60.0) as u32);
            let color = to_rgba(with_opacity(config.color, config.opacity));
            if let Ok(qr) = generate_qr(qr_data, qr_size, color) {
                let (qx, qy) = crate::pipeline::image_pipeline::qr_position(
                    px_w,
                    px_h,
                    qr.width(),
                    qr.height(),
                    config.qr_code_position,
                    config.margin,
                );
                canvas.blit(&qr, qx, qy);
            }
        }

        // Overlay custom image if -I was provided.
        if let Some(ref img_path) = config.image_path {
            let _ = overlay_image_on_canvas(&mut canvas, img_path, config);
        }

        // Apply opacity to the watermark overlay.
        let mut img =
            crate::pipeline::image_pipeline::apply_opacity(canvas.into_image(), config.opacity);

        // Apply universal perturbation for AI-removal hardening.
        crate::watermark::perturb::perturb(&mut img);

        // Apply anti-AI adversarial prompt injection.
        if config.anti_ai {
            crate::watermark::anti_ai::apply_anti_ai(&mut img, config.color, config.opacity);
        }

        // Split RGBA into RGB + alpha for PDF XObject + SMask.
        let (rgb_data, alpha_data) = split_rgba(&img);

        // Compress data with FlateDecode for much smaller file sizes.
        let rgb_compressed = deflate_compress(&rgb_data);
        let alpha_compressed = deflate_compress(&alpha_data);

        // Create the SMask (alpha channel) as a compressed grayscale image stream.
        let mut smask_dict = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"XObject".to_vec())),
            ("Subtype", Object::Name(b"Image".to_vec())),
            ("Width", Object::Integer(px_w as i64)),
            ("Height", Object::Integer(px_h as i64)),
            ("ColorSpace", Object::Name(b"DeviceGray".to_vec())),
            ("BitsPerComponent", Object::Integer(8)),
            ("Filter", Object::Name(b"FlateDecode".to_vec())),
        ]);
        smask_dict.set("Length", Object::Integer(alpha_compressed.len() as i64));
        let smask_stream = Stream::new(smask_dict, alpha_compressed);
        let smask_id = doc.add_object(smask_stream);

        // Create the image XObject with compressed RGB data + SMask reference.
        let mut img_dict = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"XObject".to_vec())),
            ("Subtype", Object::Name(b"Image".to_vec())),
            ("Width", Object::Integer(px_w as i64)),
            ("Height", Object::Integer(px_h as i64)),
            ("ColorSpace", Object::Name(b"DeviceRGB".to_vec())),
            ("BitsPerComponent", Object::Integer(8)),
            ("Filter", Object::Name(b"FlateDecode".to_vec())),
        ]);
        img_dict.set("Length", Object::Integer(rgb_compressed.len() as i64));
        img_dict.set("SMask", Object::Reference(smask_id));
        let img_stream = Stream::new(img_dict, rgb_compressed);
        let img_id = doc.add_object(img_stream);

        // Register the image as a named XObject resource on the page.
        let img_name = format!("FmWm{page_num}");
        add_xobject_resource(&mut doc, *page_id, &img_name, img_id)?;

        // Build a content stream that draws the image scaled to the full page.
        let draw_ops = format!(
            "q\n{w:.4} 0 0 {h:.4} 0 0 cm\n/{name} Do\nQ\n",
            w = page_width_pt,
            h = page_height_pt,
            name = img_name,
        );
        let draw_stream = Stream::new(Dictionary::new(), draw_ops.into_bytes());
        let draw_id = doc.add_object(draw_stream);

        insert_content_stream(&mut doc, *page_id, draw_id, config.behind)?;

        // Overlay invisible scrambled text to poison copy-paste.
        if config.copy_poison {
            let poison_ops = build_copy_poison(page_width_pt, page_height_pt, *page_num);
            let poison_stream = Stream::new(Dictionary::new(), poison_ops.into_bytes());
            let poison_id = doc.add_object(poison_stream);
            add_font_resource(&mut doc, *page_id, "FmCP", "Helvetica")?;
            insert_content_stream(&mut doc, *page_id, poison_id, false)?;
        }

        debug!("Watermarked page {page_num}");
    }

    if config.flatten {
        for (_, page_id) in &page_ids {
            flatten_page_contents(&mut doc, *page_id);
        }
    }

    doc.save(&output_path)
        .with_context(|| format!("Failed to save PDF: {}", output_path.display()))?;

    info!(
        "Watermarked PDF {} -> {}",
        input.display(),
        output_path.display()
    );
    Ok(())
}

// ── PDF-to-image conversion ─────────────────────────────────────────────────

/// Rasterize each PDF page to an image, watermark it via the image pipeline,
/// and save in the requested format. Requires `pdftoppm` (poppler-utils).
fn process_pdf_to_image(
    config: &WatermarkConfig,
    args: &CliArgs,
    output_path: &std::path::Path,
) -> anyhow::Result<()> {
    let input = &config.input;

    if config.dry_run {
        info!(
            "[dry-run] Would convert+watermark PDF {} -> {}",
            input.display(),
            output_path.display()
        );
        return Ok(());
    }

    // Count pages to decide naming.
    let doc = Document::load(input)
        .with_context(|| format!("Failed to load PDF: {}", input.display()))?;
    let page_ids: Vec<u32> = doc
        .get_pages()
        .into_iter()
        .collect::<BTreeMap<_, _>>()
        .keys()
        .copied()
        .collect();
    let page_range = &config.pages;
    let pages_to_process: Vec<u32> = page_ids
        .iter()
        .copied()
        .filter(|p| page_range.contains(*p))
        .filter(|p| {
            config
                .skip_pages
                .as_ref()
                .is_none_or(|skip| !skip.contains(*p))
        })
        .collect();
    drop(doc);

    if pages_to_process.is_empty() {
        anyhow::bail!("No pages to process after applying page range filters.");
    }

    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let multi_page = pages_to_process.len() > 1;

    for &page_num in &pages_to_process {
        let temp_png = temp_dir.join(format!("firemark_{pid}_page{page_num}.png"));

        rasterize_pdf_page(input, page_num, config.dpi, &temp_png)
            .with_context(|| format!("Failed to rasterize page {page_num}"))?;

        // Build a per-page config pointing at the rasterized image.
        let mut page_config = config.clone();
        page_config.input = temp_png.clone();

        let page_output = if multi_page {
            let stem = output_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            let ext = output_path
                .extension()
                .unwrap_or_default()
                .to_string_lossy();
            output_path.with_file_name(format!("{stem}_page{page_num}.{ext}"))
        } else {
            output_path.to_path_buf()
        };
        page_config.output = Some(page_output.clone());

        crate::pipeline::image_pipeline::process_image(&page_config, args)
            .with_context(|| format!("Failed to watermark rasterized page {page_num}"))?;

        // Clean up temp file.
        let _ = std::fs::remove_file(&temp_png);

        info!(
            "Converted+watermarked page {page_num} -> {}",
            page_output.display()
        );
    }

    Ok(())
}

/// Rasterize a single PDF page to a PNG image using an external renderer.
/// Tries `pdftoppm` (poppler-utils) first, then `sips` (macOS) as fallback.
fn rasterize_pdf_page(
    pdf_path: &std::path::Path,
    page_num: u32,
    dpi: u32,
    output_path: &std::path::Path,
) -> anyhow::Result<()> {
    let dpi_str = dpi.to_string();
    let page_str = page_num.to_string();

    // pdftoppm -singlefile adds its own extension, so strip it from the prefix.
    let prefix = output_path.with_extension("");

    // Try pdftoppm (poppler-utils) — best cross-platform option.
    let result = std::process::Command::new("pdftoppm")
        .args([
            "-png",
            "-r",
            &dpi_str,
            "-f",
            &page_str,
            "-l",
            &page_str,
            "-singlefile",
        ])
        .arg(pdf_path)
        .arg(&prefix)
        .output();

    if let Ok(out) = result {
        if out.status.success() {
            // pdftoppm creates <prefix>.png — rename if output_path differs.
            let created = prefix.with_extension("png");
            if created != *output_path && created.exists() {
                std::fs::rename(&created, output_path)?;
            }
            return Ok(());
        }
    }

    // Try sips (macOS built-in) — only handles first page.
    #[cfg(target_os = "macos")]
    {
        let result = std::process::Command::new("sips")
            .args(["-s", "format", "png"])
            .arg(pdf_path)
            .arg("--out")
            .arg(output_path)
            .output();

        if let Ok(out) = result {
            if out.status.success() && output_path.exists() {
                return Ok(());
            }
        }
    }

    anyhow::bail!(
        "No PDF renderer found. Install poppler-utils to enable PDF-to-image conversion:\n  \
         macOS:   brew install poppler\n  \
         Linux:   apt install poppler-utils\n  \
         Windows: choco install poppler"
    )
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Compress data with zlib/deflate.
fn deflate_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).expect("deflate write failed");
    encoder.finish().expect("deflate finish failed")
}

/// Split an RGBA image into separate RGB and alpha byte vectors.
fn split_rgba(img: &image::RgbaImage) -> (Vec<u8>, Vec<u8>) {
    let px_count = (img.width() * img.height()) as usize;
    let mut rgb = Vec::with_capacity(px_count * 3);
    let mut alpha = Vec::with_capacity(px_count);
    for px in img.pixels() {
        rgb.push(px[0]);
        rgb.push(px[1]);
        rgb.push(px[2]);
        alpha.push(px[3]);
    }
    (rgb, alpha)
}

/// Read page dimensions from MediaBox. Falls back to US Letter.
fn get_page_dimensions(doc: &Document, page_id: lopdf::ObjectId) -> anyhow::Result<(f32, f32)> {
    let page = doc.get_object(page_id).ok().and_then(|o| o.as_dict().ok());

    if let Some(dict) = page {
        if let Ok(Object::Array(media_box)) = dict.get(b"MediaBox") {
            if media_box.len() == 4 {
                let x0 = object_to_f32(&media_box[0]).unwrap_or(0.0);
                let y0 = object_to_f32(&media_box[1]).unwrap_or(0.0);
                let x1 = object_to_f32(&media_box[2]).unwrap_or(612.0);
                let y1 = object_to_f32(&media_box[3]).unwrap_or(792.0);
                return Ok((x1 - x0, y1 - y0));
            }
        }
    }

    warn!("Could not read MediaBox; defaulting to US Letter (612x792)");
    Ok((612.0, 792.0))
}

fn object_to_f32(obj: &Object) -> Option<f32> {
    match obj {
        Object::Integer(i) => Some(*i as f32),
        Object::Real(r) => Some(*r),
        _ => None,
    }
}

/// Register an XObject resource on a page.
fn add_xobject_resource(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    name: &str,
    xobj_id: lopdf::ObjectId,
) -> anyhow::Result<()> {
    // Determine if Resources is an indirect reference.
    let resources_id = {
        let page = doc.get_object(page_id).context("Page not found")?;
        let page_dict = page.as_dict().map_err(|_| anyhow::anyhow!("Not a dict"))?;
        match page_dict.get(b"Resources") {
            Ok(Object::Reference(id)) => Some(*id),
            _ => None,
        }
    };

    if let Some(res_id) = resources_id {
        let res_obj = doc.get_object_mut(res_id).context("Resources not found")?;
        let res_dict = res_obj
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !res_dict.has(b"XObject") {
            res_dict.set("XObject", Dictionary::new());
        }
        let xobj_entry = res_dict
            .get_mut(b"XObject")
            .map_err(|_| anyhow::anyhow!("XObject not found"))?;
        match xobj_entry {
            Object::Reference(xobj_dict_id) => {
                let xid = *xobj_dict_id;
                let xd = doc.get_object_mut(xid).context("XObject dict not found")?;
                let xd = xd
                    .as_dict_mut()
                    .map_err(|_| anyhow::anyhow!("Not a dict"))?;
                xd.set(name, Object::Reference(xobj_id));
            }
            Object::Dictionary(xd) => {
                xd.set(name, Object::Reference(xobj_id));
            }
            _ => {
                let mut new_xd = Dictionary::new();
                new_xd.set(name, Object::Reference(xobj_id));
                let res_obj2 = doc.get_object_mut(res_id).unwrap();
                let res_dict2 = res_obj2.as_dict_mut().unwrap();
                res_dict2.set("XObject", new_xd);
            }
        }
    } else {
        let page = doc.get_object_mut(page_id).context("Page not found")?;
        let page_dict = page
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !page_dict.has(b"Resources") {
            page_dict.set("Resources", Dictionary::new());
        }
        let resources = page_dict
            .get_mut(b"Resources")
            .map_err(|_| anyhow::anyhow!("Resources not found"))?
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !resources.has(b"XObject") {
            resources.set("XObject", Dictionary::new());
        }
        let xobj_dict = resources
            .get_mut(b"XObject")
            .map_err(|_| anyhow::anyhow!("XObject not found"))?
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        xobj_dict.set(name, Object::Reference(xobj_id));
    }

    Ok(())
}

/// Insert a content stream reference into the page's Contents.
fn insert_content_stream(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    stream_id: lopdf::ObjectId,
    behind: bool,
) -> anyhow::Result<()> {
    let page = doc
        .get_object_mut(page_id)
        .context("Page object not found")?;
    let page_dict = page
        .as_dict_mut()
        .map_err(|_| anyhow::anyhow!("Page is not a dictionary"))?;

    let new_ref = Object::Reference(stream_id);

    if let Ok(existing) = page_dict.get(b"Contents") {
        let mut refs: Vec<Object> = match existing.clone() {
            Object::Reference(id) => vec![Object::Reference(id)],
            Object::Array(arr) => arr,
            _ => vec![],
        };
        if behind {
            refs.insert(0, new_ref);
        } else {
            refs.push(new_ref);
        }
        page_dict.set("Contents", Object::Array(refs));
    } else {
        page_dict.set("Contents", new_ref);
    }

    Ok(())
}

/// Merge all content streams on a page into one uncompressed stream.
fn flatten_page_contents(doc: &mut Document, page_id: lopdf::ObjectId) {
    let content_ids: Vec<lopdf::ObjectId> = {
        let Ok(page) = doc.get_object(page_id) else {
            return;
        };
        let Ok(dict) = page.as_dict() else { return };
        let Ok(contents) = dict.get(b"Contents") else {
            return;
        };
        match contents {
            Object::Reference(id) => vec![*id],
            Object::Array(arr) => arr
                .iter()
                .filter_map(|o| {
                    if let Object::Reference(id) = o {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect(),
            _ => return,
        }
    };

    if content_ids.len() <= 1 {
        return;
    }

    let mut combined = Vec::new();
    for id in &content_ids {
        if let Ok(Object::Stream(ref stream)) = doc.get_object(*id) {
            match stream.decompressed_content() {
                Ok(data) => combined.extend_from_slice(&data),
                Err(_) => combined.extend_from_slice(&stream.content),
            }
            combined.push(b'\n');
        }
    }

    let mut dict = Dictionary::new();
    dict.set("Length", Object::Integer(combined.len() as i64));
    let merged = Stream::new(dict, combined);
    let merged_id = doc.add_object(merged);

    if let Ok(page) = doc.get_object_mut(page_id) {
        if let Ok(d) = page.as_dict_mut() {
            d.set("Contents", Object::Reference(merged_id));
        }
    }
}

/// Load an external image, scale it, and blit it centred on the canvas.
fn overlay_image_on_canvas(
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

    let target = (canvas.width().min(canvas.height()) as f32 * config.scale).max(32.0);
    let longest = src_canvas.width().max(src_canvas.height()) as f32;
    let factor = target / longest;
    let scaled = if (factor - 1.0).abs() > 0.01 {
        scale_canvas(&src_canvas, factor)
    } else {
        src_canvas
    };

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

/// Build a PDF content stream that renders invisible scrambled text across the
/// page using text rendering mode 3 (invisible but selectable). When a user
/// tries to copy-paste, the garbage characters are interleaved with the real
/// text, producing unusable output.
///
/// Key design choices for effective copy-paste poisoning:
/// - Font size matches typical document body text (9-12pt) so invisible chars
///   occupy the same vertical space as real chars in the text extractor.
/// - Dense vertical placement (every 4pt) ensures overlap with real text lines
///   regardless of the document's actual line spacing.
/// - Full-width lines so invisible chars span the same x-range as real text.
/// - Multiple short fragments per line at varying x offsets to interleave
///   with real words at the character level.
fn build_copy_poison(page_w: f32, page_h: f32, seed: u32) -> String {
    use std::fmt::Write;

    // Simple deterministic PRNG seeded per page so output is reproducible.
    let mut rng = seed.wrapping_mul(2654435761);
    let mut next = || -> u32 {
        rng ^= rng << 13;
        rng ^= rng >> 17;
        rng ^= rng << 5;
        rng
    };

    let mut ops = String::with_capacity(32768);
    ops.push_str("q\nBT\n3 Tr\n");

    let margin = 36.0_f32;
    let usable_w = (page_w - 2.0 * margin).max(100.0);
    let glyphs: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

    // Dense vertical pass: every 4pt to guarantee overlap with any line spacing.
    let mut y = page_h - margin;
    while y > margin {
        // Vary font size to match common document text sizes.
        let sizes = [9.0_f32, 10.0, 11.0, 12.0];
        let size = sizes[(next() as usize) % sizes.len()];
        let _ = writeln!(ops, "/FmCP {size:.0} Tf");

        // Approximate char width for this font size (Helvetica average ~0.5 em).
        let char_w = size * 0.5;

        // Place 3-5 fragments across the line at random x positions.
        let num_fragments = 3 + (next() % 3) as usize;
        for _ in 0..num_fragments {
            let x = margin + (next() as f32 % usable_w);
            // Fragment length: 4-12 chars (short enough to sit between real words).
            let len = 4 + (next() % 9) as usize;
            // Don't overflow past the right margin.
            let max_chars = ((page_w - margin - x) / char_w).max(1.0) as usize;
            let len = len.min(max_chars);

            let mut text = String::with_capacity(len);
            for _ in 0..len {
                text.push(glyphs[(next() as usize) % glyphs.len()] as char);
            }

            let _ = writeln!(ops, "1 0 0 1 {x:.1} {y:.1} Tm ({text}) Tj");
        }

        y -= 4.0;
    }

    ops.push_str("ET\nQ\n");
    ops
}

/// Register a standard Type1 font (e.g. Helvetica) as a named resource on a page.
fn add_font_resource(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    name: &str,
    base_font: &str,
) -> anyhow::Result<()> {
    let font_dict = Dictionary::from_iter(vec![
        ("Type", Object::Name(b"Font".to_vec())),
        ("Subtype", Object::Name(b"Type1".to_vec())),
        ("BaseFont", Object::Name(base_font.as_bytes().to_vec())),
    ]);
    let font_id = doc.add_object(font_dict);

    // Get or create the Resources → Font dictionary on the page.
    let resources_id = {
        let page = doc.get_object(page_id).context("Page not found")?;
        let page_dict = page.as_dict().map_err(|_| anyhow::anyhow!("Not a dict"))?;
        match page_dict.get(b"Resources") {
            Ok(Object::Reference(id)) => Some(*id),
            _ => None,
        }
    };

    if let Some(res_id) = resources_id {
        let res_obj = doc.get_object_mut(res_id).context("Resources not found")?;
        let res_dict = res_obj
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !res_dict.has(b"Font") {
            res_dict.set("Font", Dictionary::new());
        }
        let font_entry = res_dict
            .get_mut(b"Font")
            .map_err(|_| anyhow::anyhow!("Font not found"))?;
        match font_entry {
            Object::Reference(font_dict_id) => {
                let fid = *font_dict_id;
                let fd = doc.get_object_mut(fid).context("Font dict not found")?;
                let fd = fd
                    .as_dict_mut()
                    .map_err(|_| anyhow::anyhow!("Not a dict"))?;
                fd.set(name, Object::Reference(font_id));
            }
            Object::Dictionary(fd) => {
                fd.set(name, Object::Reference(font_id));
            }
            _ => {
                let mut new_fd = Dictionary::new();
                new_fd.set(name, Object::Reference(font_id));
                let res_obj2 = doc.get_object_mut(res_id).unwrap();
                let res_dict2 = res_obj2.as_dict_mut().unwrap();
                res_dict2.set("Font", new_fd);
            }
        }
    } else {
        let page = doc.get_object_mut(page_id).context("Page not found")?;
        let page_dict = page
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !page_dict.has(b"Resources") {
            page_dict.set("Resources", Dictionary::new());
        }
        let resources = page_dict
            .get_mut(b"Resources")
            .map_err(|_| anyhow::anyhow!("Resources not found"))?
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !resources.has(b"Font") {
            resources.set("Font", Dictionary::new());
        }
        let font_dict_res = resources
            .get_mut(b"Font")
            .map_err(|_| anyhow::anyhow!("Font not found"))?
            .as_dict_mut()
            .map_err(|_| anyhow::anyhow!("Not a dict"))?;

        font_dict_res.set(name, Object::Reference(font_id));
    }

    Ok(())
}
