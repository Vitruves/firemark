use rand::Rng;

use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};

/// Render a full-page intercalated text background.
///
/// Main text and secondary text alternate on every other row, creating a dense
/// security pattern.  When `opacity_factor < 1.0` the pattern is rendered more
/// subtly — use this for single-element types (badge, seal, …) that overlay
/// their primary graphic on top.
pub fn render_text_background(
    config: &WatermarkConfig,
    width: u32,
    height: u32,
    opacity_factor: f32,
) -> Result<Canvas> {
    let ctx = TemplateContext::default();
    let main_text = template::resolve(&config.main_text, &ctx);
    let secondary = template::resolve(&config.secondary_text, &ctx);

    let font = load_font(config.font.as_deref(), config.font_weight)?;

    let base_opacity = config.opacity * opacity_factor;
    // Auto-scale main text to ~12% of page width
    let scale = config.font_size.unwrap_or_else(|| {
        auto_scale(&main_text, width, config.scale.min(0.15), &font).max(14.0)
    });
    let sec_scale = scale * 0.55;

    let (tw, th) = measure_text(&font, &main_text, scale);
    let has_secondary = !secondary.is_empty();
    let (stw, _sth) = if has_secondary {
        measure_text(&font, &secondary, sec_scale)
    } else {
        (0.0, 0.0)
    };

    let h_gap = config.tile_spacing.max(20) as f32;
    let main_cell_w = tw + h_gap;
    let sec_cell_w = if has_secondary { stw + h_gap } else { main_cell_w };
    let row_h = th + h_gap * 0.5;

    let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
    let work_size = (diag * 1.5).ceil() as u32;

    let mut work = Canvas::new(work_size, work_size);
    let mut rng = rand::thread_rng();

    // Random grid phase offset — shifts entire grid origin each render.
    let phase_x: f32 = rng.gen_range(0.0..main_cell_w);
    let phase_y: f32 = rng.gen_range(0.0..row_h);

    let main_cols = (work_size as f32 / main_cell_w).ceil() as i32 + 2;
    let sec_cols = (work_size as f32 / sec_cell_w).ceil() as i32 + 2;
    let rows = (work_size as f32 / row_h).ceil() as i32 + 2;

    for row in -1..rows {
        let is_secondary_row = has_secondary && row % 2 != 0;
        let stagger = if row % 2 != 0 { main_cell_w / 2.0 } else { 0.0 };

        if is_secondary_row {
            for col in -1..sec_cols {
                // Per-cell position jitter
                let dx: f32 = rng.gen_range(-sec_cell_w * 0.03..sec_cell_w * 0.03);
                let dy: f32 = rng.gen_range(-row_h * 0.03..row_h * 0.03);
                let x = col as f32 * sec_cell_w + stagger + phase_x + dx;
                let y = row as f32 * row_h + phase_y + dy;

                // Per-cell opacity jitter
                let alpha_mult: f32 = rng.gen_range(0.92..1.0);
                let cell_color = with_opacity(config.color, base_opacity * alpha_mult);
                let cell_rgba = to_rgba(cell_color);

                work.draw_text(&font, &secondary, x, y, sec_scale, cell_rgba);
            }
        } else {
            for col in -1..main_cols {
                // Per-cell position jitter
                let dx: f32 = rng.gen_range(-main_cell_w * 0.03..main_cell_w * 0.03);
                let dy: f32 = rng.gen_range(-row_h * 0.03..row_h * 0.03);
                let x = col as f32 * main_cell_w + stagger + phase_x + dx;
                let y = row as f32 * row_h + phase_y + dy;

                // Per-cell opacity jitter
                let alpha_mult: f32 = rng.gen_range(0.92..1.0);
                let cell_color = with_opacity(config.color, base_opacity * alpha_mult);
                let cell_rgba = to_rgba(cell_color);

                work.draw_text(&font, &main_text, x, y, scale, cell_rgba);
            }
        }
    }

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
