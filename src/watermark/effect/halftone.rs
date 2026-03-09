use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Full-page dot-pattern (halftone) rendering of text.
///
/// The entire document is first covered with a dense repeating text grid (like
/// tile), rotated by `config.rotation`.  The rendered image is then converted
/// to a halftone dot pattern:
///   - The temporary canvas is sampled on a regular grid (cell size ~4-6 px).
///   - For each cell, the average alpha is computed.
///   - A filled circle is drawn with radius proportional to that average alpha.
///
/// The result looks like newspaper print or security dot-matrix printing --
/// text constructed entirely from variably-sized dots, covering the whole page.
pub struct HalftoneRenderer;

impl WatermarkRenderer for HalftoneRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx);
        let secondary = template::resolve(&config.secondary_text, &ctx);
        let has_secondary = !secondary.is_empty();

        let font = load_font(config.font.as_deref(), config.font_weight)?;

        let spacing = config.tile_spacing.max(30) as f32;
        let scale = config.font_size.unwrap_or_else(|| {
            auto_scale(&text, spacing as u32, 0.65, &font).max(10.0).min(spacing * 0.85)
        });
        let sec_scale = scale * 0.55;

        let (tw, th) = measure_text(&font, &text, scale);
        let (stw, sth) = if has_secondary {
            measure_text(&font, &secondary, sec_scale)
        } else {
            (0.0, 0.0)
        };

        if tw <= 0.0 || th <= 0.0 {
            return Ok(Canvas::new(width, height));
        }

        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // ── Step 1: Render a full-page text grid onto an oversized canvas ──

        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.6).ceil() as u32;

        let cell_w = tw + spacing;
        let cell_h = th + spacing;

        let cols = (work_size as f32 / cell_w).ceil() as i32 + 2;
        let rows = (work_size as f32 / cell_h).ceil() as i32 + 2;

        let mut work = Canvas::new(work_size, work_size);

        for row in -1..rows {
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

        // Rotate the working canvas.
        let rotated = rotate_canvas(&work, config.rotation);

        // Crop the centre to a temporary canvas at target size.
        let mut tmp = Canvas::new(width, height);
        let ox = (rotated.width() as i32 - width as i32) / 2;
        let oy = (rotated.height() as i32 - height as i32) / 2;

        let rot_src = rotated.image();
        for dy in 0..height {
            for dx in 0..width {
                let sx = ox + dx as i32;
                let sy = oy + dy as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < rotated.width()
                    && (sy as u32) < rotated.height()
                {
                    let px = *rot_src.get_pixel(sx as u32, sy as u32);
                    if px[3] > 0 {
                        tmp.set_pixel(dx as i32, dy as i32, px);
                    }
                }
            }
        }

        // ── Step 2: Convert the temporary image to halftone dots ──

        let mut rng = rand::thread_rng();
        // Random dot_spacing: [4, 7] instead of fixed 5
        let dot_spacing: u32 = rng.gen_range(4..=7);
        // Random dot grid phase: offset (0 to spacing-1) in X and Y
        let phase_x: u32 = rng.gen_range(0..dot_spacing);
        let phase_y: u32 = rng.gen_range(0..dot_spacing);
        let max_radius = (dot_spacing as f32 / 2.0) as i32;
        let tmp_img = tmp.image();

        let mut canvas = Canvas::new(width, height);

        let mut gy: u32 = phase_y;
        while gy < height {
            let mut gx: u32 = phase_x;
            while gx < width {
                // Compute average alpha in this grid cell.
                let mut alpha_sum: u32 = 0;
                let mut count: u32 = 0;

                for sy in gy..(gy + dot_spacing).min(height) {
                    for sx in gx..(gx + dot_spacing).min(width) {
                        alpha_sum += tmp_img.get_pixel(sx, sy)[3] as u32;
                        count += 1;
                    }
                }

                if count > 0 {
                    let avg_alpha = alpha_sum as f32 / count as f32;
                    if avg_alpha > 3.0 {
                        // Dot radius proportional to coverage.
                        let radius =
                            ((avg_alpha / 255.0) * max_radius as f32).ceil() as i32;
                        // Per-dot position jitter: ±1px
                        let jx: i32 = rng.gen_range(-1..=1);
                        let jy: i32 = rng.gen_range(-1..=1);
                        let dcx = gx as i32 + dot_spacing as i32 / 2 + jx;
                        let dcy = gy as i32 + dot_spacing as i32 / 2 + jy;
                        if radius > 0 {
                            canvas.fill_circle(dcx, dcy, radius, rgba);
                        }
                    }
                }

                gx += dot_spacing;
            }
            gy += dot_spacing;
        }

        Ok(canvas)
    }
}
