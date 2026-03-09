use rand::Rng;

use crate::cli::args::FontWeight;
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Full-width military stencil text spanning the page.
///
/// The main text is auto-scaled to fill the entire page width, drawn with
/// wide letter spacing and a thick outline-only effect (no fill).  Two
/// additional smaller repetitions appear at the top and bottom thirds of the
/// document for full-page coverage.
pub struct StencilRenderer;

impl WatermarkRenderer for StencilRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx).to_uppercase();

        // Always bold/black for the stencil aesthetic.
        let weight = match config.font_weight {
            FontWeight::Thin | FontWeight::Light | FontWeight::Regular => FontWeight::Bold,
            other => other,
        };
        let font = load_font(config.font.as_deref(), weight)?;

        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Wide letter spacing: 20-30% of font height — take the larger of the
        // user-specified spacing and a computed minimum.
        let margin = config.margin.max(10);
        let usable_width = width - margin * 2;

        // Scale text so that with wide spacing it fills the usable width.
        // We iterate: scale the raw text, compute total with spacing, then adjust.
        let base_scale = auto_scale(&text, usable_width, 0.90, &font);
        let spacing = config.letter_spacing.max(base_scale * 0.22);
        let char_count = text.chars().count() as f32;
        let total_spacing = spacing * (char_count - 1.0).max(0.0);
        // Adjust scale down to account for the spacing we add
        let (raw_w, _) = measure_text(&font, &text, base_scale);
        let target_w = usable_width as f32;
        let main_scale = if raw_w + total_spacing > 0.0 {
            base_scale * (target_w / (raw_w + total_spacing))
        } else {
            base_scale
        };
        let main_scale = config.font_size.unwrap_or(main_scale).max(8.0);

        // Measure per-character widths at the final scale.
        let char_widths: Vec<f32> = text
            .chars()
            .map(|ch| {
                let (cw, _) = measure_text(&font, &ch.to_string(), main_scale);
                cw
            })
            .collect();
        let (_, main_th) = measure_text(&font, &text, main_scale);

        // Recalculate spacing at the final scale.
        let final_spacing = config.letter_spacing.max(main_scale * 0.22);
        let total_rendered_w: f32 =
            char_widths.iter().sum::<f32>() + final_spacing * (char_count - 1.0).max(0.0);

        // Build a working canvas large enough for the full page + rotation slop.
        let diag = ((width as f32).powi(2) + (height as f32).powi(2)).sqrt();
        let work_size = (diag * 1.3).ceil() as u32;
        let mut work = Canvas::new(work_size, work_size);

        let cx_offset = (work_size as f32 - width as f32) / 2.0;
        let cy_offset = (work_size as f32 - height as f32) / 2.0;

        // Draw the main stencil text at the centre.
        let main_x = cx_offset + margin as f32 + (usable_width as f32 - total_rendered_w) / 2.0;
        let main_y = cy_offset + (height as f32 - main_th) / 2.0;
        draw_stencil_line(
            &mut work,
            &font,
            &text,
            &char_widths,
            main_x,
            main_y,
            main_scale,
            final_spacing,
            rgba,
        );

        // Smaller repetitions at top third and bottom third.
        let mut rng = rand::thread_rng();
        // Randomize scale ratio: 0.45 → [0.40, 0.50]
        let small_scale = main_scale * rng.gen_range(0.40..0.50);
        let small_spacing = final_spacing * 0.45;
        let small_widths: Vec<f32> = text
            .chars()
            .map(|ch| {
                let (cw, _) = measure_text(&font, &ch.to_string(), small_scale);
                cw
            })
            .collect();
        let (_, small_th) = measure_text(&font, &text, small_scale);
        let small_total_w: f32 =
            small_widths.iter().sum::<f32>() + small_spacing * (char_count - 1.0).max(0.0);

        // Top third — randomize vertical position: 0.18 → [0.15, 0.21]
        let top_x = cx_offset + (width as f32 - small_total_w) / 2.0;
        let top_y = cy_offset + height as f32 * rng.gen_range(0.15..0.21) - small_th / 2.0;
        draw_stencil_line(
            &mut work,
            &font,
            &text,
            &small_widths,
            top_x,
            top_y,
            small_scale,
            small_spacing,
            rgba,
        );

        // Bottom third — randomize vertical position: 0.82 → [0.79, 0.85]
        let bot_y = cy_offset + height as f32 * rng.gen_range(0.79..0.85) - small_th / 2.0;
        draw_stencil_line(
            &mut work,
            &font,
            &text,
            &small_widths,
            top_x,
            bot_y,
            small_scale,
            small_spacing,
            rgba,
        );

        // --- Intercalated secondary text rows between the 3 main rows ---
        let secondary = template::resolve(&config.secondary_text, &ctx);
        if !secondary.is_empty() {
            let sec_text = secondary.to_uppercase();
            let sec_scale = main_scale * 0.30;
            let sec_spacing = final_spacing * 0.30;
            let sec_char_count = sec_text.chars().count() as f32;
            let sec_widths: Vec<f32> = sec_text
                .chars()
                .map(|ch| {
                    let (cw, _) = measure_text(&font, &ch.to_string(), sec_scale);
                    cw
                })
                .collect();
            let (_, sec_th) = measure_text(&font, &sec_text, sec_scale);
            let sec_total_w: f32 =
                sec_widths.iter().sum::<f32>() + sec_spacing * (sec_char_count - 1.0).max(0.0);

            let sec_color = with_opacity(config.color, config.opacity * 0.6);
            let sec_rgba = to_rgba(sec_color);

            // Between top row and centre row
            let mid_top_y = cy_offset + height as f32 * 0.36 - sec_th / 2.0;
            let sec_x = cx_offset + (width as f32 - sec_total_w) / 2.0;
            draw_stencil_line(
                &mut work,
                &font,
                &sec_text,
                &sec_widths,
                sec_x,
                mid_top_y,
                sec_scale,
                sec_spacing,
                sec_rgba,
            );

            // Between centre row and bottom row
            let mid_bot_y = cy_offset + height as f32 * 0.64 - sec_th / 2.0;
            draw_stencil_line(
                &mut work,
                &font,
                &sec_text,
                &sec_widths,
                sec_x,
                mid_bot_y,
                sec_scale,
                sec_spacing,
                sec_rgba,
            );
        }

        // Apply rotation (default -5 for stencil).
        let angle = if (config.rotation + 45.0).abs() < 0.5 {
            -5.0
        } else {
            config.rotation
        };
        let rotated = rotate_canvas(&work, angle);

        // Crop the centre to the target dimensions.
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

/// Draw a single line of stencil text — outline-only with wide letter spacing.
///
/// The outline effect is achieved by drawing the text at several small offsets
/// (N, S, E, W, and diagonals) from the nominal position, creating a thick
/// outline silhouette.  The interior is then "erased" by drawing again at the
/// exact position with a fully transparent colour, but since the font rasteriser
/// is consistent the result is a clean outline.
///
/// Because the canvas is transparent to start, we take a simpler approach:
/// draw the text multiple times with 1-2 px offsets in all 8 compass
/// directions to build up the outline.
fn draw_stencil_line(
    canvas: &mut Canvas,
    font: &ab_glyph::FontArc,
    text: &str,
    char_widths: &[f32],
    start_x: f32,
    y: f32,
    scale: f32,
    spacing: f32,
    color: image::Rgba<u8>,
) {
    // Outline offsets — 2 pixel spread in 8 directions.
    let offsets: &[(f32, f32)] = &[
        (-2.0, 0.0),
        (2.0, 0.0),
        (0.0, -2.0),
        (0.0, 2.0),
        (-1.5, -1.5),
        (1.5, -1.5),
        (-1.5, 1.5),
        (1.5, 1.5),
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
    ];

    let mut rng = rand::thread_rng();
    for &(ox, oy) in offsets {
        let mut cx = start_x + ox;
        for (i, ch) in text.chars().enumerate() {
            let s = ch.to_string();
            // Per-character y-drop: ±2px
            let char_dy: f32 = rng.gen_range(-2.0..2.0);
            canvas.draw_text(font, &s, cx, y + oy + char_dy, scale, color);
            cx += char_widths[i] + spacing;
        }
    }
}
