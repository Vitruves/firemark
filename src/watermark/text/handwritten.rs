use rand::Rng;

use crate::cli::args::Position;
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::background::render_text_background;
use crate::watermark::renderer::WatermarkRenderer;

/// Signature-style watermark with a decorative wavy underline and optional
/// secondary text.
///
/// Designed to look like a hand-signed annotation on a document, complete
/// with a small flourish mark at the start and a flowing underline.
pub struct HandwrittenRenderer;

impl WatermarkRenderer for HandwrittenRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx);
        let secondary = template::resolve(&config.secondary_text, &ctx);

        let font = load_font(config.font.as_deref(), config.font_weight)?;

        // Scale to 30-40% of canvas width.
        let target_ratio = config.scale.clamp(0.25, 0.50);
        let scale = config.font_size.unwrap_or_else(|| {
            auto_scale(&text, width, target_ratio, &font)
        });

        let (tw, th) = measure_text(&font, &text, scale);
        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Secondary text metrics.
        let sec_scale = scale * 0.45;
        let (sec_tw, sec_th) = if !secondary.is_empty() {
            measure_text(&font, &secondary, sec_scale)
        } else {
            (0.0, 0.0)
        };

        // Build a tight canvas for the signature composition.
        let flourish_w = (scale * 0.4) as u32;
        let underline_extra = (tw * 0.15) as u32; // underline extends past text
        let pad = 30u32;

        let comp_w = flourish_w + tw.ceil() as u32 + underline_extra + pad * 2;
        let underline_gap = (th * 0.25) as u32;
        let comp_h = th.ceil() as u32 + underline_gap + sec_th.ceil() as u32 + pad * 2 + 20;

        let mut comp = Canvas::new(comp_w, comp_h);

        let text_x = pad as f32 + flourish_w as f32;
        let text_y = pad as f32;

        let mut rng = rand::thread_rng();

        // --- Flourish mark at the start (a small "x" / cross) ---
        let fl_cx = pad as f32 + flourish_w as f32 * 0.4;
        let fl_cy = text_y + th * 0.5;
        // Random flourish stroke lengths: ±15%
        let fl_r = (scale * 0.12).max(4.0) * rng.gen_range(0.85..1.15);
        let fl_color = with_opacity(config.color, config.opacity * 0.7);
        let fl_rgba = to_rgba(fl_color);
        comp.draw_thick_line(
            (fl_cx - fl_r) as i32,
            (fl_cy - fl_r) as i32,
            (fl_cx + fl_r) as i32,
            (fl_cy + fl_r) as i32,
            2,
            fl_rgba,
        );
        comp.draw_thick_line(
            (fl_cx + fl_r) as i32,
            (fl_cy - fl_r) as i32,
            (fl_cx - fl_r) as i32,
            (fl_cy + fl_r) as i32,
            2,
            fl_rgba,
        );

        // --- Main text with per-character baseline wobble ---
        {
            let mut cx = text_x;
            for ch in text.chars() {
                let s = ch.to_string();
                let (cw, _) = measure_text(&font, &s, scale);
                let dy: f32 = rng.gen_range(-1.5..1.5);
                comp.draw_text(&font, &s, cx, text_y + dy, scale, rgba);
                cx += cw;
            }
        }

        // --- Wavy underline ---
        let ul_y_base = text_y + th + underline_gap as f32 * 0.5;
        let ul_start_x = text_x - flourish_w as f32 * 0.3;
        let ul_end_x = text_x + tw + underline_extra as f32;
        let wave_amplitude = (th * 0.06).max(2.0);
        let wave_segments = 40u32;
        let segment_len = (ul_end_x - ul_start_x) / wave_segments as f32;

        let ul_color = with_opacity(config.color, config.opacity * 0.8);
        let ul_rgba = to_rgba(ul_color);

        // Random wave phase
        let phase: f32 = rng.gen_range(0.0..2.0 * std::f32::consts::PI);
        for i in 0..wave_segments {
            let x1 = ul_start_x + i as f32 * segment_len;
            let x2 = ul_start_x + (i + 1) as f32 * segment_len;
            // Per-segment wave amplitude noise: ±20%
            let amp1 = wave_amplitude * rng.gen_range(0.8..1.2);
            let amp2 = wave_amplitude * rng.gen_range(0.8..1.2);
            let y1 = ul_y_base + amp1 * (i as f32 * 0.5 + phase).sin();
            let y2 = ul_y_base + amp2 * ((i + 1) as f32 * 0.5 + phase).sin();
            comp.draw_thick_line(x1 as i32, y1 as i32, x2 as i32, y2 as i32, 2, ul_rgba);
        }

        // --- Secondary text below the underline ---
        if !secondary.is_empty() {
            let sec_x = text_x + tw - sec_tw; // right-aligned with main text
            let sec_y = ul_y_base + wave_amplitude + 6.0;
            let sec_color = with_opacity(config.color, config.opacity * 0.65);
            let sec_rgba = to_rgba(sec_color);
            comp.draw_text(&font, &secondary, sec_x, sec_y, sec_scale, sec_rgba);
        }

        // --- Rotate slightly ---
        let angle = if (config.rotation + 45.0).abs() < 0.5 {
            -5.0
        } else {
            config.rotation
        };
        let rotated = rotate_canvas(&comp, angle);

        // --- Render subtle intercalated text background behind the signature ---
        let mut canvas = render_text_background(config, width, height, 0.25)?;
        let margin = config.margin as i32;

        let (ox, oy) = match config.position {
            Position::TopLeft => (margin, margin),
            Position::TopRight => (width as i32 - rotated.width() as i32 - margin, margin),
            Position::BottomLeft => (margin, height as i32 - rotated.height() as i32 - margin),
            Position::BottomRight => (
                width as i32 - rotated.width() as i32 - margin,
                height as i32 - rotated.height() as i32 - margin,
            ),
            Position::Center | Position::Tile => (
                (width as i32 - rotated.width() as i32) / 2,
                (height as i32 - rotated.height() as i32) / 2,
            ),
        };

        canvas.blit(&rotated, ox + config.offset.0, oy + config.offset.1);

        Ok(canvas)
    }
}
