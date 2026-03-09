use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Full-page grainy / distressed text pattern.
///
/// The entire document is covered with a dense repeating text grid (like tile),
/// rotated by `config.rotation`.  After rendering, per-pixel noise is applied
/// to every non-transparent pixel:
///   - Random RGB channel variation: +-40
///   - Random alpha variation: +-20
///   - ~15% of non-transparent pixels are randomly deleted (set to transparent)
///
/// The result resembles a photocopy of a photocopy -- the text is clearly there
/// but degraded and textured, making it extremely difficult to cleanly remove.
pub struct NoiseRenderer;

impl WatermarkRenderer for NoiseRenderer {
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

        // Oversized working canvas for full coverage after rotation.
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

        // Rotate the whole sheet.
        let rotated = rotate_canvas(&work, config.rotation);

        // Crop to target size.
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

        // Apply per-pixel noise and random deletion to all non-transparent pixels.
        let mut rng = rand::thread_rng();
        // Vary noise_rgb in [30, 60] and delete_chance in [0.10, 0.25] per render
        let noise_rgb: i16 = rng.gen_range(30..=60);
        let noise_alpha: i16 = 20;
        let delete_chance: f32 = rng.gen_range(0.10..0.25);

        let snapshot = canvas.image().clone();
        for y in 0..height {
            for x in 0..width {
                let px = snapshot.get_pixel(x, y);
                if px[3] == 0 {
                    continue;
                }

                // Random deletion -- set pixel fully transparent.
                if rng.gen::<f32>() < delete_chance {
                    canvas.set_pixel(x as i32, y as i32, image::Rgba([0, 0, 0, 0]));
                    continue;
                }

                // Random RGB and alpha noise.
                let nr: i16 = rng.gen_range(-noise_rgb..=noise_rgb);
                let ng: i16 = rng.gen_range(-noise_rgb..=noise_rgb);
                let nb: i16 = rng.gen_range(-noise_rgb..=noise_rgb);
                let na: i16 = rng.gen_range(-noise_alpha..=noise_alpha);

                let r = (px[0] as i16 + nr).clamp(0, 255) as u8;
                let g = (px[1] as i16 + ng).clamp(0, 255) as u8;
                let b = (px[2] as i16 + nb).clamp(0, 255) as u8;
                let a = (px[3] as i16 + na).clamp(0, 255) as u8;

                canvas.set_pixel(x as i32, y as i32, image::Rgba([r, g, b, a]));
            }
        }

        Ok(canvas)
    }
}
