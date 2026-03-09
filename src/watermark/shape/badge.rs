use rand::Rng;

use crate::cli::args::{FontWeight, Position};
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::template::{self, TemplateContext};
use crate::watermark::background::render_text_background;
use crate::watermark::renderer::WatermarkRenderer;

/// Security shield/badge emblem with decorative star and dual text lines.
pub struct BadgeRenderer;

impl WatermarkRenderer for BadgeRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let main_text = template::resolve(&config.main_text, &ctx);
        let secondary_text = template::resolve(&config.secondary_text, &ctx);

        let bold_font = load_font(config.font.as_deref(), FontWeight::Bold)?;
        let regular_font = load_font(config.font.as_deref(), FontWeight::Regular)?;

        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Darker shade for the inner border
        let dark_color = with_opacity(
            [
                (config.color[0] as f32 * 0.6) as u8,
                (config.color[1] as f32 * 0.6) as u8,
                (config.color[2] as f32 * 0.6) as u8,
                config.color[3],
            ],
            config.opacity,
        );
        let dark_rgba = to_rgba(dark_color);

        let white = to_rgba(with_opacity([255, 255, 255, 255], config.opacity));
        let white_dim = to_rgba(with_opacity([255, 255, 255, 200], config.opacity));

        // Shield size: scale directly controls proportion of page (default 0.4 = 40% of page)
        let dim = width.min(height) as f32;
        let shield_h = (dim * config.scale * 0.85).ceil().max(80.0) as i32;
        let shield_w = (shield_h as f32 * 0.8) as i32;

        // Determine placement center based on position
        let margin = config.margin as i32;
        let (cx, cy) = match config.position {
            Position::TopLeft => (margin + shield_w / 2, margin + shield_h / 2),
            Position::TopRight => (width as i32 - margin - shield_w / 2, margin + shield_h / 2),
            Position::BottomLeft => (margin + shield_w / 2, height as i32 - margin - shield_h / 2),
            Position::BottomRight => (
                width as i32 - margin - shield_w / 2,
                height as i32 - margin - shield_h / 2,
            ),
            _ => (width as i32 / 2, height as i32 / 2),
        };

        let mut canvas = render_text_background(config, width, height, 0.3)?;
        let mut rng = rand::thread_rng();

        // Random shield position: ±3px from center
        let cx = cx + rng.gen_range(-3..=3);
        let cy = cy + rng.gen_range(-3..=3);

        // ── Build shield polygon ──
        let hw = shield_w / 2;
        let hh = shield_h / 2;
        let shoulder_inset = hw / 6;

        let base_shield_points = vec![
            (cx - hw + shoulder_inset, cy - hh),
            (cx + hw - shoulder_inset, cy - hh),
            (cx + hw, cy - hh + shoulder_inset),
            (cx + hw, cy - hh / 4),
            (cx + hw - hw / 8, cy + hh / 4),
            (cx + hw / 3, cy + hh - hh / 6),
            (cx, cy + hh),
            (cx - hw / 3, cy + hh - hh / 6),
            (cx - hw + hw / 8, cy + hh / 4),
            (cx - hw, cy - hh / 4),
            (cx - hw, cy - hh + shoulder_inset),
        ];

        // Per-vertex jitter: ±1-2px
        let shield_points: Vec<(i32, i32)> = base_shield_points
            .iter()
            .map(|&(px, py)| {
                (
                    px + rng.gen_range(-2..=2),
                    py + rng.gen_range(-2..=2),
                )
            })
            .collect();

        // Fill the shield
        canvas.fill_polygon(&shield_points, rgba);

        // ── Inner border ──
        // Create a slightly inset version of the shield polygon for the inner border line
        let inset = 4i32;
        let inner_points: Vec<(i32, i32)> = shield_points
            .iter()
            .map(|&(px, py)| {
                // Move each point toward the center by `inset` pixels
                let dx = cx - px;
                let dy = cy - py;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < 1.0 {
                    (px, py)
                } else {
                    let factor = inset as f32 / dist;
                    (
                        px + (dx as f32 * factor) as i32,
                        py + (dy as f32 * factor) as i32,
                    )
                }
            })
            .collect();

        canvas.draw_polygon(&inner_points, dark_rgba);
        // Draw a second pass slightly more inset for a double-line effect
        let inner2_points: Vec<(i32, i32)> = shield_points
            .iter()
            .map(|&(px, py)| {
                let dx = cx - px;
                let dy = cy - py;
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                if dist < 1.0 {
                    (px, py)
                } else {
                    let factor = (inset + 2) as f32 / dist;
                    (
                        px + (dx as f32 * factor) as i32,
                        py + (dy as f32 * factor) as i32,
                    )
                }
            })
            .collect();
        canvas.draw_polygon(&inner2_points, dark_rgba);

        // ── Star at the top of the shield ──
        let star_y = cy - hh + shield_h / 6 + rng.gen_range(-1..=1);
        let star_outer = (shield_w as f32 * 0.10).ceil() as i32;
        let star_inner = (star_outer as f32 * 0.45) as i32;
        canvas.fill_star(cx, star_y, star_outer, star_inner, 5, white);

        // ── Main text (bold, large, centered in upper-middle area) ──
        let main_scale = config.font_size.unwrap_or_else(|| {
            auto_scale(&main_text, shield_w as u32, 0.70, &bold_font)
        });
        let (mtw, mth) = measure_text(&bold_font, &main_text, main_scale);
        // Position main text in the center of the shield, slightly above vertical midpoint
        let text_y = cy as f32 - mth * 0.3;
        let text_x = cx as f32 - mtw / 2.0;
        canvas.draw_text(&bold_font, &main_text, text_x, text_y, main_scale, white);

        // ── Secondary text (smaller, below main text) ──
        if !secondary_text.is_empty() {
            let sec_scale = main_scale * 0.45;
            let (stw, _sth) = measure_text(&regular_font, &secondary_text, sec_scale);
            let sec_x = cx as f32 - stw / 2.0;
            let sec_y = text_y + mth + mth * 0.15;
            canvas.draw_text(
                &regular_font,
                &secondary_text,
                sec_x,
                sec_y,
                sec_scale,
                white_dim,
            );
        }

        // ── Thin decorative line below the main text area ──
        let line_y = (text_y + mth + mth * 0.05) as i32;
        let line_hw = (shield_w as f32 * 0.25) as i32;
        canvas.draw_line(cx - line_hw, line_y, cx + line_hw, line_y, white_dim);

        // ── Small flanking stars beside the main text ──
        let flank_star_r = (main_scale * 0.12).ceil() as i32;
        let flank_star_ir = (flank_star_r as f32 * 0.45) as i32;
        let flank_y = (text_y + mth / 2.0) as i32;
        if flank_star_r > 1 {
            canvas.fill_star(
                (text_x - main_scale * 0.3) as i32,
                flank_y,
                flank_star_r,
                flank_star_ir,
                5,
                white_dim,
            );
            canvas.fill_star(
                (text_x + mtw + main_scale * 0.3) as i32,
                flank_y,
                flank_star_r,
                flank_star_ir,
                5,
                white_dim,
            );
        }

        Ok(canvas)
    }
}
