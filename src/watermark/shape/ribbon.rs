use rand::Rng;

use crate::cli::args::{FontWeight, Position};
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::background::render_text_background;
use crate::watermark::renderer::WatermarkRenderer;

/// Diagonal corner ribbon banner with fold triangles and border lines.
pub struct RibbonRenderer;

impl WatermarkRenderer for RibbonRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let main_text = template::resolve(&config.main_text, &ctx);

        let bold_font = load_font(config.font.as_deref(), FontWeight::Bold)?;

        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Darker shade for ribbon edges and fold triangles
        let dark_color = with_opacity(
            [
                (config.color[0] as f32 * 0.55) as u8,
                (config.color[1] as f32 * 0.55) as u8,
                (config.color[2] as f32 * 0.55) as u8,
                config.color[3],
            ],
            config.opacity,
        );
        let dark_rgba = to_rgba(dark_color);

        // Even darker for the fold shadow triangles
        let fold_color = with_opacity(
            [
                (config.color[0] as f32 * 0.35) as u8,
                (config.color[1] as f32 * 0.35) as u8,
                (config.color[2] as f32 * 0.35) as u8,
                config.color[3],
            ],
            config.opacity,
        );
        let fold_rgba = to_rgba(fold_color);

        let white = to_rgba(with_opacity([255, 255, 255, 255], config.opacity));

        // Ribbon band width scales with page — substantial enough to read
        let dim = width.min(height) as f32;
        let ribbon_band_h = (dim * 0.09).ceil().max(44.0) as u32;
        // Corner span: how far the ribbon extends along each page edge from the corner.
        // scale=0.4 → ~28% of page dim → compact corner banner.
        let corner_span = (dim * config.scale * 0.7).ceil().max(100.0) as u32;
        // The ribbon strip length needs to span the diagonal between the two edge
        // endpoints: length = corner_span * sqrt(2) * 1.3 (extra room for text padding).
        let ribbon_len = (corner_span as f32 * 1.42 * 1.3).ceil() as u32;

        // ── Build the ribbon on its own horizontal canvas ──
        let mut ribbon = Canvas::new(ribbon_len, ribbon_band_h);

        // Fill the main ribbon band
        ribbon.fill_rect(0, 0, ribbon_len, ribbon_band_h, rgba);

        // Draw darker border lines along top and bottom edges of the ribbon
        let mut rng = rand::thread_rng();
        // Random border thickness: 1-3px instead of fixed 2
        let border_thickness: u32 = rng.gen_range(1..=3);
        for i in 0..border_thickness {
            ribbon.draw_line(
                0,
                i as i32,
                ribbon_len as i32,
                i as i32,
                dark_rgba,
            );
            ribbon.draw_line(
                0,
                (ribbon_band_h - 1 - i) as i32,
                ribbon_len as i32,
                (ribbon_band_h - 1 - i) as i32,
                dark_rgba,
            );
        }

        // ── Center the text on the ribbon ──
        // Scale text to fit within the band height, not the full length
        let text_scale = config.font_size.unwrap_or_else(|| {
            (ribbon_band_h as f32 * 0.55).max(14.0)
        });
        let (tw, th) = measure_text(&bold_font, &main_text, text_scale);
        let tx = (ribbon_len as f32 - tw) / 2.0;
        let ty = (ribbon_band_h as f32 - th) / 2.0;
        ribbon.draw_text(&bold_font, &main_text, tx, ty, text_scale, white);

        // ── Embed the ribbon strip in a square canvas before rotating ──
        // rotate_about_center keeps the same dimensions, so a narrow strip
        // would lose its ends.  Embedding in a square canvas sized to the
        // strip's diagonal avoids all clipping.
        let strip_diag = ((ribbon_len as f32).powi(2) + (ribbon_band_h as f32).powi(2))
            .sqrt()
            .ceil() as u32
            + 2;
        let mut padded = Canvas::new(strip_diag, strip_diag);
        let pad_x = (strip_diag as i32 - ribbon_len as i32) / 2;
        let pad_y = (strip_diag as i32 - ribbon_band_h as i32) / 2;
        padded.blit(&ribbon, pad_x, pad_y);

        let rotation = config.rotation;
        let rotated = rotate_canvas(&padded, rotation);

        // ── Create the output canvas and determine corner placement ──
        let mut canvas = render_text_background(config, width, height, 0.3)?;

        // Random fold triangle size: ±10%
        let base_fold_size = (ribbon_band_h as f32 * 0.35).ceil();
        let fold_size = (base_fold_size * rng.gen_range(0.90..1.10)).ceil() as i32;

        // Determine which corner to place the ribbon in
        let is_top = matches!(
            config.position,
            Position::TopLeft | Position::TopRight | Position::Center
        );
        let is_right = matches!(
            config.position,
            Position::TopRight | Position::BottomRight | Position::Center
        );

        // Calculate the offset to place the rotated ribbon centered on the corner diagonal.
        // The ribbon center (where text sits) should land at the midpoint of the
        // diagonal segment from one page edge to the other at the chosen corner.
        let rw = rotated.width() as i32;
        let rh = rotated.height() as i32;
        let cs2 = corner_span as i32 / 2; // half corner span = diagonal midpoint offset
        let (ox, oy) = if is_top && is_right {
            (width as i32 - cs2 - rw / 2, cs2 - rh / 2)
        } else if is_top {
            (cs2 - rw / 2, cs2 - rh / 2)
        } else if is_right {
            (width as i32 - cs2 - rw / 2, height as i32 - cs2 - rh / 2)
        } else {
            (cs2 - rw / 2, height as i32 - cs2 - rh / 2)
        };

        // ── Draw fold triangles at the ribbon endpoints ──
        // The folds are small dark triangles where the ribbon meets the page edges,
        // creating a 3D "tucked behind" effect.
        if is_top && is_right {
            // Top edge fold: small triangle hanging below the ribbon at the top
            let fold_top = vec![
                (width as i32 - corner_span as i32, 0),
                (width as i32 - corner_span as i32 + fold_size, 0),
                (width as i32 - corner_span as i32, fold_size),
            ];
            canvas.fill_polygon(&fold_top, fold_rgba);
            // Right edge fold
            let fold_right = vec![
                (width as i32, corner_span as i32 - fold_size),
                (width as i32, corner_span as i32),
                (width as i32 - fold_size, corner_span as i32),
            ];
            canvas.fill_polygon(&fold_right, fold_rgba);
        } else if is_top {
            let fold_top = vec![
                (corner_span as i32 - fold_size, 0),
                (corner_span as i32, 0),
                (corner_span as i32, fold_size),
            ];
            canvas.fill_polygon(&fold_top, fold_rgba);
            let fold_left = vec![
                (0, corner_span as i32 - fold_size),
                (0, corner_span as i32),
                (fold_size, corner_span as i32),
            ];
            canvas.fill_polygon(&fold_left, fold_rgba);
        } else if is_right {
            let fold_bottom = vec![
                (width as i32 - corner_span as i32, height as i32),
                (width as i32 - corner_span as i32 + fold_size, height as i32),
                (width as i32 - corner_span as i32 + fold_size, height as i32 - fold_size),
            ];
            canvas.fill_polygon(&fold_bottom, fold_rgba);
            let fold_right = vec![
                (width as i32, height as i32 - corner_span as i32),
                (width as i32, height as i32 - corner_span as i32 + fold_size),
                (width as i32 - fold_size, height as i32 - corner_span as i32),
            ];
            canvas.fill_polygon(&fold_right, fold_rgba);
        } else {
            let fold_bottom = vec![
                (corner_span as i32 - fold_size, height as i32),
                (corner_span as i32, height as i32),
                (corner_span as i32 - fold_size, height as i32 - fold_size),
            ];
            canvas.fill_polygon(&fold_bottom, fold_rgba);
            let fold_left = vec![
                (0, height as i32 - corner_span as i32),
                (fold_size, height as i32 - corner_span as i32),
                (0, height as i32 - corner_span as i32 + fold_size),
            ];
            canvas.fill_polygon(&fold_left, fold_rgba);
        }

        // ── Blit the rotated ribbon onto the canvas with random offset ±2px ──
        let ribbon_jx: i32 = rng.gen_range(-2..=2);
        let ribbon_jy: i32 = rng.gen_range(-2..=2);
        canvas.blit(&rotated, ox + ribbon_jx, oy + ribbon_jy);

        Ok(canvas)
    }
}
