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

/// Professional circular notary/corporate seal with rings, decorative dots,
/// arc text, and centered main text.
pub struct SealRenderer;

impl WatermarkRenderer for SealRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let main_text = template::resolve(&config.main_text, &ctx);
        let secondary_text = template::resolve(&config.secondary_text, &ctx);

        let bold_font = load_font(config.font.as_deref(), FontWeight::Bold)?;
        let regular_font = load_font(config.font.as_deref(), FontWeight::Regular)?;

        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Slightly lighter shade for decorative elements
        let light_color = with_opacity(
            [
                (config.color[0] as u16 + (255 - config.color[0] as u16) / 3) as u8,
                (config.color[1] as u16 + (255 - config.color[1] as u16) / 3) as u8,
                (config.color[2] as u16 + (255 - config.color[2] as u16) / 3) as u8,
                config.color[3],
            ],
            config.opacity,
        );
        let light_rgba = to_rgba(light_color);

        // Seal diameter: ~32% of the smaller page dimension
        let dim = width.min(height) as f32;
        let seal_diameter = (dim * 0.32 * config.scale).ceil() as i32;
        let radius = seal_diameter / 2;

        // Determine placement center based on position
        let margin = config.margin as i32;
        let (cx, cy) = match config.position {
            Position::TopLeft => (margin + radius, margin + radius),
            Position::TopRight => (width as i32 - margin - radius, margin + radius),
            Position::BottomLeft => (margin + radius, height as i32 - margin - radius),
            Position::BottomRight => (
                width as i32 - margin - radius,
                height as i32 - margin - radius,
            ),
            _ => (width as i32 / 2, height as i32 / 2),
        };

        let mut canvas = render_text_background(config, width, height, 0.3)?;
        let mut rng = rand::thread_rng();

        // ── Outermost ring: thick circle (3-4px) with micro-variation ──
        let outer_thickness = 4u32;
        // Random ring radii micro-variation: ±1px
        let outer_r_jitter: i32 = rng.gen_range(-1..=1);
        canvas.draw_thick_circle(cx, cy, radius + outer_r_jitter, outer_thickness, rgba);

        // ── Gap, then inner ring: thin circle ──
        let ring_gap = (radius as f32 * 0.06).ceil() as i32;
        let inner_ring_r = radius - ring_gap as i32 - outer_thickness as i32 / 2 + rng.gen_range(-1..=1);
        canvas.draw_circle(cx, cy, inner_ring_r, rgba);

        // ── Decorative elements between the two rings ──
        // Random number of decorative stars: 8-12 instead of fixed 10
        let num_decorations: u32 = rng.gen_range(8..=12);
        let deco_radius = (inner_ring_r as f32 + ring_gap as f32 / 2.0
            + outer_thickness as f32 / 2.0) as f32;
        let deco_star_outer = (ring_gap as f32 * 0.45).ceil() as i32;
        let deco_star_inner = (deco_star_outer as f32 * 0.45) as i32;

        // Random starting angle for star ring
        let star_ring_start: f32 = rng.gen_range(0.0..std::f32::consts::PI * 2.0);
        for i in 0..num_decorations {
            let angle = std::f32::consts::PI * 2.0 * i as f32 / num_decorations as f32
                - std::f32::consts::FRAC_PI_2 + star_ring_start;
            let dx = cx as f32 + deco_radius * angle.cos();
            let dy = cy as f32 + deco_radius * angle.sin();
            if deco_star_outer > 1 {
                canvas.fill_star(
                    dx as i32,
                    dy as i32,
                    deco_star_outer,
                    deco_star_inner,
                    5,
                    light_rgba,
                );
            } else {
                canvas.fill_circle(dx as i32, dy as i32, 2, light_rgba);
            }
        }

        // ── Secondary text curved along the TOP arc (inside the inner ring) ──
        if !secondary_text.is_empty() {
            let arc_r = inner_ring_r as f32 * 0.82;
            let arc_scale = (inner_ring_r as f32 * 0.14).max(8.0);
            // Top arc: start angle pointing upward (-PI/2)
            canvas.draw_text_on_arc(
                &regular_font,
                &secondary_text,
                cx as f32,
                cy as f32,
                arc_r,
                -std::f32::consts::FRAC_PI_2,
                arc_scale,
                rgba,
            );
        }

        // ── Bottom arc: separator dots ──
        {
            let separator = "\u{2022}  \u{2022}  \u{2022}";
            let arc_r = inner_ring_r as f32 * 0.82;
            let dot_scale = (inner_ring_r as f32 * 0.10).max(6.0);
            canvas.draw_text_on_arc(
                &regular_font,
                separator,
                cx as f32,
                cy as f32,
                arc_r,
                std::f32::consts::FRAC_PI_2,
                dot_scale,
                rgba,
            );
        }

        // ── Main text: bold, large, centered ──
        let main_scale = config.font_size.unwrap_or_else(|| {
            auto_scale(&main_text, (inner_ring_r as u32) * 2, 0.55, &bold_font)
        });
        let (mtw, mth) = measure_text(&bold_font, &main_text, main_scale);
        let mtx = cx as f32 - mtw / 2.0;
        let mty = cy as f32 - mth / 2.0;
        canvas.draw_text(&bold_font, &main_text, mtx, mty, main_scale, rgba);

        // ── Thin decorative line below center text ──
        let line_y = (mty + mth + mth * 0.2) as i32;
        let line_hw = (inner_ring_r as f32 * 0.35) as i32;
        canvas.draw_line(cx - line_hw, line_y, cx + line_hw, line_y, light_rgba);

        // ── Small flanking stars beside the main text ──
        let flank_r = (main_scale * 0.15).ceil() as i32;
        let flank_ir = (flank_r as f32 * 0.45) as i32;
        let flank_y = (mty + mth / 2.0) as i32;
        if flank_r > 1 {
            let flank_x_left = (mtx - main_scale * 0.35) as i32;
            let flank_x_right = (mtx + mtw + main_scale * 0.35) as i32;
            canvas.fill_star(flank_x_left, flank_y, flank_r, flank_ir, 5, rgba);
            canvas.fill_star(flank_x_right, flank_y, flank_r, flank_ir, 5, rgba);
        }

        // ── Innermost thin circle enclosing the center area ──
        let center_ring_r = (inner_ring_r as f32 * 0.55) as i32;
        canvas.draw_circle(cx, cy, center_ring_r, light_rgba);

        Ok(canvas)
    }
}
