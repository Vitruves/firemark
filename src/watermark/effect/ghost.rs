use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Full-page very subtle embossed repeating text pattern.
///
/// Covers the entire document with a dense grid of text at very low opacity
/// (clamped to at most 0.15).  Each text instance is drawn twice with a small
/// pixel offset to create a raised/3D emboss effect:
///   1. A lighter highlight shifted (-1, -1) -- simulates light from top-left.
///   2. A darker shadow shifted (+1, +1) -- simulates the cast shadow.
///
/// The effect is barely visible when viewing the document normally but becomes
/// obvious when inspecting closely or photocopying.
pub struct GhostRenderer;

impl WatermarkRenderer for GhostRenderer {
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

        let mut rng = rand::thread_rng();

        // Ghost uses very low opacity -- clamp to at most 0.15.
        let ghost_opacity = config.opacity.min(0.15);

        // Randomize emboss direction: angle from random direction
        let emboss_angle: f32 = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
        // Randomize offset magnitude: [0.8, 1.5] px
        let emboss_mag: f32 = rng.gen_range(0.8..1.5);
        let hi_dx = -emboss_mag * emboss_angle.cos();
        let hi_dy = -emboss_mag * emboss_angle.sin();
        let sh_dx = emboss_mag * emboss_angle.cos();
        let sh_dy = emboss_mag * emboss_angle.sin();

        // Build colour variants for the emboss effect.
        let [r, g, b, _] = config.color;

        // Highlight: shift RGB towards white.
        let hi_r = ((r as u16 + 60).min(255)) as u8;
        let hi_g = ((g as u16 + 60).min(255)) as u8;
        let hi_b = ((b as u16 + 60).min(255)) as u8;
        // Shadow: shift RGB towards black.
        let sh_r = r.saturating_sub(60);
        let sh_g = g.saturating_sub(60);
        let sh_b = b.saturating_sub(60);

        // Oversized working canvas for full-page coverage after rotation.
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

                // Per-cell opacity jitter: ±0.02 around ghost_opacity
                let cell_opacity = (ghost_opacity + rng.gen_range(-0.02_f32..0.02)).clamp(0.01, 0.20);
                let cell_hi = with_opacity([hi_r, hi_g, hi_b, 255], cell_opacity);
                let cell_hi_rgba = to_rgba(cell_hi);
                let cell_sh = with_opacity([sh_r, sh_g, sh_b, 255], cell_opacity);
                let cell_sh_rgba = to_rgba(cell_sh);

                // 1. Highlight pass -- randomized direction.
                work.draw_text(&font, draw_text, x + hi_dx, y + hi_dy, draw_scale, cell_hi_rgba);

                // 2. Shadow pass -- randomized direction.
                work.draw_text(&font, draw_text, x + sh_dx, y + sh_dy, draw_scale, cell_sh_rgba);
            }
        }

        // Rotate and crop to target.
        let rotated = rotate_canvas(&work, config.rotation);

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
