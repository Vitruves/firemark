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
pub fn process_pdf(config: &WatermarkConfig, _args: &CliArgs) -> anyhow::Result<()> {
    let input = &config.input;
    let output_path = resolve_output_path(
        input,
        config.output.as_deref(),
        config.suffix.as_deref(),
    );

    if config.dry_run {
        info!(
            "[dry-run] Would watermark PDF {} -> {}",
            input.display(),
            output_path.display()
        );
        return Ok(());
    }

    debug!("Loading PDF: {}", input.display());
    let mut doc =
        Document::load(input).with_context(|| format!("Failed to load PDF: {}", input.display()))?;

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
        let filigrane = render_filigrane(px_w, px_h, config.color, 0.18, config.filigrane);
        canvas.blit(&filigrane, 0, 0);

        // Overlay QR code if --qr-data was provided.
        if let Some(ref qr_data) = config.qr_data {
            let qr_size = (px_w.min(px_h) as f32 * config.scale * 0.5).max(60.0) as u32;
            let color = to_rgba(with_opacity(config.color, config.opacity));
            if let Ok(qr) = generate_qr(qr_data, qr_size, color) {
                let qx = (px_w as i32 - qr.width() as i32) / 2;
                let qy = (px_h as i32 - qr.height() as i32) / 2;
                canvas.blit(&qr, qx, qy);
            }
        }

        // Overlay custom image if -I was provided.
        if let Some(ref img_path) = config.image_path {
            let _ = overlay_image_on_canvas(&mut canvas, img_path, config);
        }

        let mut img = canvas.into_image();

        // Apply universal perturbation for AI-removal hardening.
        crate::watermark::perturb::perturb(&mut img);

        // Apply anti-AI adversarial prompt injection.
        if config.anti_ai {
            crate::watermark::anti_ai::apply_anti_ai(&mut img, config.color);
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
    let page = doc
        .get_object(page_id)
        .ok()
        .and_then(|o| o.as_dict().ok());

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
        Object::Real(r) => Some(*r as f32),
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
        let res_dict = res_obj.as_dict_mut().map_err(|_| anyhow::anyhow!("Not a dict"))?;

        if !res_dict.has(b"XObject") {
            res_dict.set("XObject", Dictionary::new());
        }
        let xobj_entry = res_dict.get_mut(b"XObject").map_err(|_| anyhow::anyhow!("XObject not found"))?;
        match xobj_entry {
            Object::Reference(xobj_dict_id) => {
                let xid = *xobj_dict_id;
                let xd = doc.get_object_mut(xid).context("XObject dict not found")?;
                let xd = xd.as_dict_mut().map_err(|_| anyhow::anyhow!("Not a dict"))?;
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
        let page_dict = page.as_dict_mut().map_err(|_| anyhow::anyhow!("Not a dict"))?;

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
        let Ok(page) = doc.get_object(page_id) else { return };
        let Ok(dict) = page.as_dict() else { return };
        let Ok(contents) = dict.get(b"Contents") else { return };
        match contents {
            Object::Reference(id) => vec![*id],
            Object::Array(arr) => arr
                .iter()
                .filter_map(|o| if let Object::Reference(id) = o { Some(*id) } else { None })
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
