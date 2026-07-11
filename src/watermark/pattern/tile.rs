use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Dense, full-page regular grid of repeated rotated text.
///
/// Covers the ENTIRE canvas with a uniform grid of text instances, each rotated
/// at `config.rotation`.  An oversized working canvas is filled first, then
/// rotated and centre-cropped so that even the corners are fully covered.
/// This produces the classic "CONFIDENTIAL" security watermark seen on leaked
/// or restricted documents -- dense, regular, and impossible to crop away.
pub struct TileRenderer;

impl WatermarkRenderer for TileRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx);
        let secondary = template::resolve(&config.secondary_text, &ctx);
        let has_secondary = !secondary.is_empty();

        let font = load_font(config.font.as_deref(), config.font_weight)?;

        // Choose a compact font size so we get many repetitions.
        // If tile_spacing is set, derive from that; otherwise use a fraction of
        // the page width so the text is small and repeated many times.
        let spacing = config.tile_spacing.max(60) as f32;
        let scale = config.font_size.unwrap_or_else(|| {
            // Text should be readable — auto-scale to ~80% of tile cell width
            auto_scale(&text, spacing as u32, 0.80, &font)
                .max(14.0)
                .min(spacing * 0.95)
        });
        let sec_scale = scale * 0.55;

        let (tw, th) = measure_text(&font, &text, scale);
        let (stw, sth) = if has_secondary {
            measure_text(&font, &secondary, sec_scale)
        } else {
            (0.0, 0.0)
        };
        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        if tw <= 0.0 || th <= 0.0 {
            return Ok(Canvas::new(width, height));
        }

        // Grid cell dimensions.
        let cell_w = if let Some(cols) = config.tile_cols {
            width as f32 / cols as f32
        } else {
            tw + spacing
        };
        let cell_h = if let Some(rows) = config.tile_rows {
            height as f32 / rows as f32
        } else {
            th + spacing
        };

        // Build an oversized working canvas so that after rotation the target
        // area is fully covered.  The diagonal of the target gives the minimum
        // size; we use 1.6x for a comfortable margin.
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.6).ceil() as u32;

        let mut work = Canvas::new(work_size, work_size);

        let cols = (work_size as f32 / cell_w).ceil() as i32 + 2;
        let rows = (work_size as f32 / cell_h).ceil() as i32 + 2;

        for row in -1..rows {
            // Stagger every other row by half a cell to break vertical alignment.
            let x_stagger = if row % 2 != 0 { cell_w / 2.0 } else { 0.0 };

            // Alternate between main and secondary text on odd rows.
            let use_secondary = has_secondary && row % 2 != 0;
            let draw_text = if use_secondary { &secondary } else { &text };
            let draw_scale = if use_secondary { sec_scale } else { scale };
            let draw_tw = if use_secondary { stw } else { tw };
            let draw_th = if use_secondary { sth } else { th };

            for col in -1..cols {
                let x = col as f32 * cell_w + x_stagger + (cell_w - draw_tw) / 2.0;
                let y = row as f32 * cell_h + (cell_h - draw_th) / 2.0;
                work.draw_text(&font, draw_text, x, y, draw_scale, rgba);
            }
        }

        // Rotate the whole sheet by the configured angle.
        let rotated = rotate_canvas(&work, config.rotation);

        // Crop the centre to match the target dimensions.
        let mut canvas = Canvas::new(width, height);
        let ox = (rotated.width() as i32 - width as i32) / 2;
        let oy = (rotated.height() as i32 - height as i32) / 2;

        let src = rotated.image();
        for dy in 0..height {
            for dx in 0..width {
                let sx = ox + dx as i32;
                let sy = oy + dy as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < rotated.width()
                    && (sy as u32) < rotated.height()
                {
                    let px = *src.get_pixel(sx as u32, sy as u32);
                    if px[3] > 0 {
                        canvas.blend_pixel(dx as i32, dy as i32, px);
                    }
                }
            }
        }

        Ok(canvas)
    }
}
