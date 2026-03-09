use crate::cli::args::FontWeight;
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

use rand::Rng;

/// Multiple thick black redaction bars spanning the full document width,
/// with white text repeated across each bar.
///
/// Produces 5-8 prominent horizontal bars at irregular vertical positions,
/// each bar filled black at high opacity with the watermark text rendered
/// in white on top, repeated across the bar width.  Bars have slight random
/// height variation and some are slightly tilted for a realistic redacted
/// document look.
pub struct RedactedRenderer;

impl WatermarkRenderer for RedactedRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx).to_uppercase();
        let secondary = template::resolve(&config.secondary_text, &ctx).to_uppercase();
        let has_secondary = !secondary.is_empty();

        let weight = match config.font_weight {
            FontWeight::Thin | FontWeight::Light | FontWeight::Regular => FontWeight::Bold,
            other => other,
        };
        let font = load_font(config.font.as_deref(), weight)?;

        let margin = config.margin.max(8);

        // Font size — moderate so text fits comfortably inside the bars.
        let scale = config.font_size.unwrap_or_else(|| {
            (height as f32 * 0.028).clamp(14.0, 36.0)
        });
        let sec_scale = scale * 0.7;

        let (tw, th) = measure_text(&font, &text, scale);
        let (sec_tw, sec_th) = if has_secondary {
            measure_text(&font, &secondary, sec_scale)
        } else {
            (tw, th)
        };

        // Bar opacity — high (default 0.85+).
        let bar_opacity = config.opacity.max(0.85);
        let bar_color = with_opacity(config.color, bar_opacity);
        let bar_rgba = to_rgba(bar_color);

        // Text on bars is white with the same opacity.
        let text_color = with_opacity([255, 255, 255, 255], bar_opacity);
        let text_rgba = to_rgba(text_color);

        let mut canvas = Canvas::new(width, height);
        let mut rng = rand::thread_rng();

        // Determine the number of bars (5-8).
        let num_bars = rng.gen_range(5u32..=8);

        // Distribute bars across the vertical span with some randomisation.
        let usable_h = height as f32 - margin as f32 * 2.0;
        let nominal_bar_h = (th * 1.8).max(scale * 2.0);
        let spacing = usable_h / num_bars as f32;

        let bar_width = width - margin * 2;

        for i in 0..num_bars {
            // Random left-edge start ±5px
            let bar_x_start = margin as i32 + rng.gen_range(-5..=5);
            // Determine if this bar uses secondary text (alternating bars).
            let is_secondary_bar = has_secondary && i % 2 != 0;
            let bar_text = if is_secondary_bar { &secondary } else { &text };
            let bar_scale = if is_secondary_bar { sec_scale } else { scale };
            let bar_tw = if is_secondary_bar { sec_tw } else { tw };
            let bar_th = if is_secondary_bar { sec_th } else { th };

            // Vertical centre of this bar, with slight random offset.
            let nominal_y = margin as f32 + spacing * (i as f32 + 0.5);
            let y_jitter = rng.gen_range(-spacing * 0.15..spacing * 0.15);
            let bar_cy = nominal_y + y_jitter;

            // Slight random height variation (±20%).
            let h_var: f32 = rng.gen_range(0.80..1.20);
            let bar_h = (nominal_bar_h * h_var).max(bar_th + 6.0);

            let bar_y = (bar_cy - bar_h / 2.0) as i32;

            // Some bars get a slight tilt (±1-2 degrees).
            let tilt: f32 = if rng.gen_bool(0.4) {
                rng.gen_range(-2.0f32..2.0)
            } else {
                0.0
            };

            if tilt.abs() > 0.3 {
                // Draw the bar on a mini canvas, rotate, then blit.
                let extra = (bar_width as f32 * tilt.to_radians().tan().abs()) as u32 + 10;
                let mini_h = bar_h.ceil() as u32 + extra;
                let mini_w = bar_width + extra;
                let mut mini = Canvas::new(mini_w, mini_h);

                let local_bar_y = (mini_h as f32 - bar_h) as i32 / 2;
                mini.fill_rect(0, local_bar_y, mini_w, bar_h.ceil() as u32, bar_rgba);

                // Repeat text across the bar.
                // Random text gap in bar: 0.4-0.8 × bar_tw
                let text_gap = bar_tw * rng.gen_range(0.4..0.8);
                let mut tx = margin as f32 * 0.5;
                let ty = local_bar_y as f32 + (bar_h - bar_th) / 2.0;
                while tx < mini_w as f32 {
                    mini.draw_text(&font, bar_text, tx, ty, bar_scale, text_rgba);
                    tx += bar_tw + text_gap;
                }

                let rotated = rotate_canvas(&mini, tilt);
                let blit_x =
                    bar_x_start - (rotated.width() as i32 - bar_width as i32) / 2;
                let blit_y = bar_y - (rotated.height() as i32 - bar_h.ceil() as i32) / 2;
                canvas.blit(&rotated, blit_x, blit_y);
            } else {
                // Draw straight bar directly on the output canvas.
                canvas.fill_rect(
                    bar_x_start,
                    bar_y,
                    bar_width,
                    bar_h.ceil() as u32,
                    bar_rgba,
                );

                // Repeat text across the bar with random gap.
                let text_gap = bar_tw * rng.gen_range(0.4..0.8);
                let mut tx = bar_x_start as f32 + margin as f32 * 0.5;
                let ty = bar_y as f32 + (bar_h - bar_th) / 2.0;
                while tx < (bar_x_start + bar_width as i32) as f32 {
                    canvas.draw_text(&font, bar_text, tx, ty, bar_scale, text_rgba);
                    tx += bar_tw + text_gap;
                }
            }
        }

        Ok(canvas)
    }
}
