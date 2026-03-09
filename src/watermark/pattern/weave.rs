use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Interlocking diagonal text rows that cross each other, covering the full page.
///
/// Two sets of parallel text rows are drawn:
///   Set 1: rows running top-left to bottom-right (+45 degrees)
///   Set 2: rows running top-right to bottom-left (-45 degrees)
///
/// Where the rows cross they create a woven/textile appearance reminiscent of
/// security printing on banknotes and certificates.  Set 2 is drawn at 70%
/// opacity so the two layers remain visually distinguishable.
pub struct WeaveRenderer;

impl WatermarkRenderer for WeaveRenderer {
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

        let base_color = config.color;
        let base_opacity = config.opacity;

        let mut canvas = Canvas::new(width, height);
        let mut rng = rand::thread_rng();

        // Each "set" is a full-page grid at a different angle.
        // We build each set on an oversized canvas, rotate, and crop.
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.6).ceil() as u32;

        let cell_w = tw + spacing;
        let cell_h = th + spacing;

        let cols = (work_size as f32 / cell_w).ceil() as i32 + 2;
        let rows = (work_size as f32 / cell_h).ceil() as i32 + 2;

        // Set 1: randomize angle in [42, 48] degrees.
        let angle1: f32 = rng.gen_range(42.0..48.0);
        let color1 = with_opacity(base_color, base_opacity);
        let rgba1 = to_rgba(color1);

        let mut work1 = Canvas::new(work_size, work_size);
        for row in -1..rows {
            let x_stagger = if row % 2 != 0 { cell_w / 2.0 } else { 0.0 };
            for col in -1..cols {
                let x = col as f32 * cell_w + x_stagger;
                let y = row as f32 * cell_h;
                work1.draw_text(&font, &text, x, y, scale, rgba1);
            }
        }

        let rotated1 = rotate_canvas(&work1, angle1);
        let ox1 = (rotated1.width() as i32 - width as i32) / 2;
        let oy1 = (rotated1.height() as i32 - height as i32) / 2;
        let src1 = rotated1.image();

        for dy in 0..height {
            for dx in 0..width {
                let sx = ox1 + dx as i32;
                let sy = oy1 + dy as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < rotated1.width()
                    && (sy as u32) < rotated1.height()
                {
                    let px = *src1.get_pixel(sx as u32, sy as u32);
                    if px[3] > 0 {
                        canvas.blend_pixel(dx as i32, dy as i32, px);
                    }
                }
            }
        }

        // Set 2: randomize angle in [-48, -42] degrees with randomized opacity [0.60, 0.80].
        let angle2: f32 = rng.gen_range(-48.0..-42.0);
        let set2_opacity: f32 = rng.gen_range(0.60..0.80);
        let color2 = with_opacity(base_color, base_opacity * set2_opacity);
        let rgba2 = to_rgba(color2);

        let text2 = if has_secondary { &secondary } else { &text };
        let scale2 = if has_secondary { sec_scale } else { scale };
        let tw2 = if has_secondary { stw } else { tw };
        let th2 = if has_secondary { sth } else { th };

        let cell_w2 = tw2 + spacing;
        let cell_h2 = th2 + spacing;
        let cols2 = (work_size as f32 / cell_w2).ceil() as i32 + 2;
        let rows2 = (work_size as f32 / cell_h2).ceil() as i32 + 2;

        let mut work2 = Canvas::new(work_size, work_size);
        for row in -1..rows2 {
            let x_stagger = if row % 2 != 0 { cell_w2 / 2.0 } else { 0.0 };
            for col in -1..cols2 {
                let x = col as f32 * cell_w2 + x_stagger;
                let y = row as f32 * cell_h2;
                work2.draw_text(&font, text2, x, y, scale2, rgba2);
            }
        }

        let rotated2 = rotate_canvas(&work2, angle2);
        let ox2 = (rotated2.width() as i32 - width as i32) / 2;
        let oy2 = (rotated2.height() as i32 - height as i32) / 2;
        let src2 = rotated2.image();

        for dy in 0..height {
            for dx in 0..width {
                let sx = ox2 + dx as i32;
                let sy = oy2 + dy as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < rotated2.width()
                    && (sy as u32) < rotated2.height()
                {
                    let px = *src2.get_pixel(sx as u32, sy as u32);
                    if px[3] > 0 {
                        canvas.blend_pixel(dx as i32, dy as i32, px);
                    }
                }
            }
        }

        Ok(canvas)
    }
}
