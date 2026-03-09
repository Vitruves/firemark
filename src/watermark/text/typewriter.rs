use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

use rand::Rng;

/// Multi-line repeated text covering the entire page like a typewritten
/// document overlay.
///
/// Uses a monospace font.  Each character has slight random x/y jitter and
/// per-character alpha variation for a vintage typewriter feel.  Faint
/// horizontal ruled lines are drawn under each row to evoke lined paper.
pub struct TypewriterRenderer;

impl WatermarkRenderer for TypewriterRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx);
        let secondary = template::resolve(&config.secondary_text, &ctx);
        let has_secondary = !secondary.is_empty();

        // Prefer mono font for the typewriter look.
        let font_name = config.font.as_deref().unwrap_or("mono");
        let font = load_font(Some(font_name), config.font_weight)?;

        // Use a moderate size so many lines fit on the page.
        let scale = config.font_size.unwrap_or_else(|| {
            (height as f32 * 0.025).clamp(12.0, 28.0)
        });
        let sec_scale = scale * 0.7;

        let base_color = with_opacity(config.color, config.opacity);
        let (_, th) = measure_text(&font, &text, scale);
        let (_, sec_th) = if has_secondary {
            measure_text(&font, &secondary, sec_scale)
        } else {
            (0.0, th)
        };

        // Build a full line by repeating the text with a separator.
        let separator = " \u{00B7} "; // middle dot

        // Measure per-character widths for the main repeated phrase.
        let phrase = format!("{}{}", text, separator);
        let phrase_chars: Vec<char> = phrase.chars().collect();
        let phrase_widths: Vec<f32> = phrase_chars
            .iter()
            .map(|ch| {
                let (cw, _) = measure_text(&font, &ch.to_string(), scale);
                cw
            })
            .collect();

        // Measure per-character widths for the secondary repeated phrase.
        let sec_phrase = format!("{}{}", secondary, separator);
        let sec_phrase_chars: Vec<char> = sec_phrase.chars().collect();
        let sec_phrase_widths: Vec<f32> = sec_phrase_chars
            .iter()
            .map(|ch| {
                let (cw, _) = measure_text(&font, &ch.to_string(), sec_scale);
                cw
            })
            .collect();

        let margin = config.margin.max(10);
        let usable_w = width - margin * 2;
        let line_spacing = (th * 1.5).max(scale * 1.6);
        let sec_line_spacing = (sec_th * 1.5).max(sec_scale * 1.6);

        // Work canvas large enough to survive slight rotation.
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.15).ceil() as u32;
        let mut work = Canvas::new(work_size, work_size);

        let cx_offset = (work_size as f32 - width as f32) / 2.0;
        let cy_offset = (work_size as f32 - height as f32) / 2.0;

        let mut rng = rand::thread_rng();
        // Widen jitter from ±4% to ±8% of scale
        let jitter = (scale * 0.08).max(1.0);
        let sec_jitter = (sec_scale * 0.08).max(1.0);

        // Faint line color — much lower opacity (base, per-line variation applied later).
        let line_base_opacity = config.opacity * 0.15;

        // Calculate total rows — if secondary text, rows alternate (main, sec, main, sec...).
        // Each pair occupies line_spacing + sec_line_spacing vertical space.
        let effective_spacing = if has_secondary {
            (line_spacing + sec_line_spacing) / 2.0
        } else {
            line_spacing
        };
        let rows = ((height as f32 + effective_spacing) / effective_spacing).ceil() as i32 + 2;
        let start_y = cy_offset + margin as f32;

        let mut y_cursor = start_y - effective_spacing; // start slightly above

        for row in -1..rows {
            let is_secondary_row = has_secondary && row % 2 != 0;
            let current_scale;
            let current_th;
            let current_jitter;
            let current_phrase_chars;
            let current_phrase_widths;
            let current_line_spacing;

            if is_secondary_row {
                current_scale = sec_scale;
                current_th = sec_th;
                current_jitter = sec_jitter;
                current_phrase_chars = &sec_phrase_chars;
                current_phrase_widths = &sec_phrase_widths;
                current_line_spacing = sec_line_spacing;
            } else {
                current_scale = scale;
                current_th = th;
                current_jitter = jitter;
                current_phrase_chars = &phrase_chars;
                current_phrase_widths = &phrase_widths;
                current_line_spacing = line_spacing;
            }

            // Add row-level Y jitter
            let row_y_jitter: f32 = rng.gen_range(-2.0..2.0);
            let base_y = y_cursor + row_y_jitter;
            y_cursor += current_line_spacing;

            // Faint ruled line under this text row with random opacity
            let line_opacity_mult: f32 = rng.gen_range(0.5..1.0);
            let rule_y = (base_y + current_th + 4.0) as i32;
            let line_color = with_opacity(config.color, line_base_opacity * line_opacity_mult);
            let line_rgba = to_rgba(line_color);
            work.draw_line(
                (cx_offset + margin as f32) as i32,
                rule_y,
                (cx_offset + margin as f32 + usable_w as f32) as i32,
                rule_y,
                line_rgba,
            );

            // Draw characters across the full width, repeating the phrase.
            let mut cx = cx_offset + margin as f32;
            let limit_x = cx_offset + margin as f32 + usable_w as f32;
            let mut char_idx = 0usize;

            while cx < limit_x {
                let pi = char_idx % current_phrase_chars.len();
                let ch = current_phrase_chars[pi];
                let cw = current_phrase_widths[pi];

                // Per-character jitter
                let dx: f32 = rng.gen_range(-current_jitter..current_jitter);
                let dy: f32 = rng.gen_range(-current_jitter..current_jitter);

                // Per-character alpha variation (±12%)
                let alpha_var: f32 = rng.gen_range(0.88..1.0);
                let opacity_mul = if is_secondary_row { 0.7 } else { 1.0 };
                let char_color = with_opacity(
                    [base_color[0], base_color[1], base_color[2], 255],
                    config.opacity * alpha_var * opacity_mul,
                );
                let char_rgba = to_rgba(char_color);

                let s = ch.to_string();
                work.draw_text(&font, &s, cx + dx, base_y + dy, current_scale, char_rgba);

                cx += cw;
                char_idx += 1;
            }
        }

        // Apply slight rotation (default -2 for typewriter).
        let angle = if (config.rotation + 45.0).abs() < 0.5 {
            -2.0
        } else {
            config.rotation
        };
        let rotated = rotate_canvas(&work, angle);

        // Crop to target.
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
