use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Full-page mosaic of text with random per-instance variation.
///
/// Like `TileRenderer` this covers the entire document, but each text instance
/// receives random perturbation in rotation (base +-15 deg), position (+-30%
/// of tile spacing), and opacity (+-20% of base).  The result is an organic,
/// chaotic-looking pattern that is still dense enough to be impossible to crop
/// or erase, providing a strong forensic watermark.
pub struct MosaicRenderer;

impl WatermarkRenderer for MosaicRenderer {
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

        // Grid cell size.
        let cell_w = if config.tile_cols.is_some() {
            width as f32 / config.tile_cols.unwrap() as f32
        } else {
            tw + spacing
        };
        let cell_h = if config.tile_rows.is_some() {
            height as f32 / config.tile_rows.unwrap() as f32
        } else {
            th + spacing
        };

        // Oversized working canvas to survive the base rotation.
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.6).ceil() as u32;

        let mut work = Canvas::new(work_size, work_size);

        let cols = (work_size as f32 / cell_w).ceil() as i32 + 2;
        let rows = (work_size as f32 / cell_h).ceil() as i32 + 2;

        let mut rng = rand::thread_rng();

        // Pad for individual tile canvases (room for rotation).
        let pad = (scale * 0.6).ceil() as u32;
        let tile_w = tw.ceil() as u32 + pad * 2;
        let tile_h = th.ceil() as u32 + pad * 2;

        // Pad for secondary tile canvases (room for rotation).
        let sec_tile_w = if has_secondary { stw.ceil() as u32 + pad * 2 } else { tile_w };
        let sec_tile_h = if has_secondary { sth.ceil() as u32 + pad * 2 } else { tile_h };

        for row in -1..rows {
            // Alternate between main and secondary text on odd rows.
            let use_secondary = has_secondary && row % 2 != 0;
            let draw_text = if use_secondary { &secondary } else { &text };
            let draw_scale = if use_secondary { sec_scale } else { scale };
            let cur_tile_w = if use_secondary { sec_tile_w } else { tile_w };
            let cur_tile_h = if use_secondary { sec_tile_h } else { tile_h };

            for col in -1..cols {
                // Random offsets: +-30% of spacing in each axis.
                let dx: f32 = rng.gen_range(-spacing * 0.3..spacing * 0.3);
                let dy: f32 = rng.gen_range(-spacing * 0.3..spacing * 0.3);

                // Random rotation: base angle +-15 degrees.
                let angle: f32 = config.rotation + rng.gen_range(-15.0..15.0);

                // Random opacity: base +-20%.
                let opacity_var: f32 = rng.gen_range(-0.20..0.20);
                let tile_opacity = (base_opacity + opacity_var).clamp(0.05, 1.0);
                let color = with_opacity(base_color, tile_opacity);
                let rgba = to_rgba(color);

                // Per-tile scale jitter ±5%
                let scale_jitter: f32 = rng.gen_range(0.95..1.05);
                let jittered_scale = draw_scale * scale_jitter;

                // Render this tile's text on a small canvas and rotate it.
                let mut tile = Canvas::new(cur_tile_w, cur_tile_h);
                tile.draw_text(
                    &font,
                    draw_text,
                    pad as f32,
                    pad as f32,
                    jittered_scale,
                    rgba,
                );

                let rotated_tile = rotate_canvas(&tile, angle);

                // Blit the rotated tile into the working canvas.
                let cx = col as f32 * cell_w + cell_w / 2.0 + dx;
                let cy = row as f32 * cell_h + cell_h / 2.0 + dy;
                let bx = cx as i32 - rotated_tile.width() as i32 / 2;
                let by = cy as i32 - rotated_tile.height() as i32 / 2;

                work.blit(&rotated_tile, bx, by);
            }
        }

        // Crop the centre region to the target dimensions.
        let mut canvas = Canvas::new(width, height);
        let ox = (work.width() as i32 - width as i32) / 2;
        let oy = (work.height() as i32 - height as i32) / 2;

        let src = work.image();
        for dy in 0..height {
            for dx in 0..width {
                let sx = ox + dx as i32;
                let sy = oy + dy as i32;
                if sx >= 0
                    && sy >= 0
                    && (sx as u32) < work.width()
                    && (sy as u32) < work.height()
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
