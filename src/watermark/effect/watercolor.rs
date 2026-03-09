use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Full-page soft blurred text pattern with watercolour-like bleeding.
///
/// The entire page is filled with a repeating text grid (like tile), rotated by
/// `config.rotation`, then the whole result is Gaussian-blurred with a large
/// sigma (15-25% of font scale) to produce soft, bleeding edges.  Each tile
/// also receives slight random colour variation (+-10% brightness) so the wash
/// looks hand-painted rather than mechanical.
pub struct WatercolorRenderer;

impl WatermarkRenderer for WatercolorRenderer {
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
        // Draw at higher opacity since blur will spread and dilute it
        let base_opacity = (config.opacity * 2.5).min(1.0);

        // Oversized working canvas for full coverage after rotation.
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.6).ceil() as u32;

        let cell_w = tw + spacing;
        let cell_h = th + spacing;

        let cols = (work_size as f32 / cell_w).ceil() as i32 + 2;
        let rows = (work_size as f32 / cell_h).ceil() as i32 + 2;

        let mut work = Canvas::new(work_size, work_size);
        let mut rng = rand::thread_rng();

        for row in -1..rows {
            let x_stagger = if row % 2 != 0 { cell_w / 2.0 } else { 0.0 };

            // Alternate between main and secondary text on odd rows.
            let use_secondary = has_secondary && row % 2 != 0;
            let draw_text = if use_secondary { &secondary } else { &text };
            let draw_scale = if use_secondary { sec_scale } else { scale };
            let draw_tw = if use_secondary { stw } else { tw };
            let draw_th = if use_secondary { sth } else { th };

            for col in -1..cols {
                // Random brightness variation: +-10% + hue jitter per tile
                let brightness: f32 = rng.gen_range(-0.10..0.10);
                // Hue jitter: slight independent variation per channel
                let hue_r: f32 = rng.gen_range(-0.03..0.03);
                let hue_g: f32 = rng.gen_range(-0.03..0.03);
                let hue_b: f32 = rng.gen_range(-0.03..0.03);
                let r = (base_color[0] as f32 * (1.0 + brightness + hue_r)).clamp(0.0, 255.0) as u8;
                let g = (base_color[1] as f32 * (1.0 + brightness + hue_g)).clamp(0.0, 255.0) as u8;
                let b = (base_color[2] as f32 * (1.0 + brightness + hue_b)).clamp(0.0, 255.0) as u8;
                let tile_color = with_opacity([r, g, b, 255], base_opacity);
                let rgba = to_rgba(tile_color);

                let x = col as f32 * cell_w + x_stagger + (cell_w - draw_tw) / 2.0;
                let y = row as f32 * cell_h + (cell_h - draw_th) / 2.0;
                work.draw_text(&font, draw_text, x, y, draw_scale, rgba);
            }
        }

        // Rotate the filled working canvas.
        let rotated = rotate_canvas(&work, config.rotation);

        // Apply Gaussian blur for soft watercolour edges.
        // Randomize blur sigma: [0.08, 0.18] of scale instead of fixed 0.12
        let sigma = (scale * rng.gen_range(0.08..0.18)).clamp(1.0, 15.0);
        let blurred = imageproc::filter::gaussian_blur_f32(rotated.image(), sigma);
        let blurred_canvas = Canvas::from_image(blurred);

        // Crop the centre to the target dimensions.
        let mut canvas = Canvas::new(width, height);
        let ox = (blurred_canvas.width() as i32 - width as i32) / 2;
        let oy = (blurred_canvas.height() as i32 - height as i32) / 2;

        let src = blurred_canvas.image();
        for dy in 0..height {
            for dx in 0..width {
                let sx = ox + dx as i32;
                let sy = oy + dy as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < blurred_canvas.width()
                    && (sy as u32) < blurred_canvas.height()
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
