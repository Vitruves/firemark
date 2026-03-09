use rand::Rng;

use crate::cli::args::FontWeight;
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::renderer::WatermarkRenderer;

/// Decorative full-page border frame with repeated text along all four sides,
/// double border lines, and ornamental corner elements. Resembles the text
/// borders found on certificates and currency.
pub struct FrameRenderer;

impl WatermarkRenderer for FrameRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx);
        let secondary_text = template::resolve(&config.secondary_text, &ctx);

        let font = load_font(config.font.as_deref(), config.font_weight)?;

        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Slightly lighter shade for the inner border and decorations
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

        let margin = config.margin.max(10) as i32;
        let text_scale = config.font_size.unwrap_or(14.0);

        let (tw, th) = measure_text(&font, &text, text_scale);

        let mut canvas = Canvas::new(width, height);

        // If the measured text width is zero, bail out to avoid infinite loops.
        if tw <= 0.0 || th <= 0.0 {
            return Ok(canvas);
        }

        // ── Outer border: thick line around the page ──
        let outer_thickness = 3u32;
        let outer_x = margin;
        let outer_y = margin;
        let outer_w = width as i32 - margin * 2;
        let outer_h = height as i32 - margin * 2;
        if outer_w > 0 && outer_h > 0 {
            for i in 0..outer_thickness as i32 {
                canvas.draw_rect(
                    outer_x + i,
                    outer_y + i,
                    (outer_w - i * 2) as u32,
                    (outer_h - i * 2) as u32,
                    rgba,
                );
            }
        }

        // ── Inner border: thinner line a few pixels inside ──
        let inner_gap = (th.ceil() as i32) + 8; // space for text between borders
        let inner_x = outer_x + outer_thickness as i32 + inner_gap;
        let inner_y = outer_y + outer_thickness as i32 + inner_gap;
        let inner_w = outer_w - (outer_thickness as i32 + inner_gap) * 2;
        let inner_h = outer_h - (outer_thickness as i32 + inner_gap) * 2;
        if inner_w > 0 && inner_h > 0 {
            canvas.draw_rect(inner_x, inner_y, inner_w as u32, inner_h as u32, rgba);
            // Second thin line 2px further in for double-line effect
            if inner_w > 4 && inner_h > 4 {
                canvas.draw_rect(
                    inner_x + 2,
                    inner_y + 2,
                    (inner_w - 4) as u32,
                    (inner_h - 4) as u32,
                    light_rgba,
                );
            }
        }

        // ── Text channel: the strip between outer and inner borders ──
        // Text is drawn in the gap between the two border rectangles.
        let text_band_y_top = outer_y + outer_thickness as i32 + 3; // just inside outer border
        let text_band_y_bot = inner_y + inner_h + 3;
        let text_band_x_left = outer_x + outer_thickness as i32 + 3;
        let text_band_x_right = inner_x + inner_w + 3;

        let separator = "  \u{2022}  "; // bullet separator between text repeats
        let (sep_w, _) = measure_text(&font, separator, text_scale);
        let step = tw + sep_w;

        // ── Top edge: text running left to right ──
        {
            let mut x = text_band_x_left as f32 + 4.0;
            let y = text_band_y_top as f32;
            let x_limit = (inner_x + inner_w) as f32 - tw;
            while x < x_limit {
                canvas.draw_text(&font, &text, x, y, text_scale, rgba);
                x += step;
            }
        }

        // ── Bottom edge: text running left to right (visually inverted by rotating 180) ──
        {
            // Render text segments into small canvases, rotate 180, and blit
            let segment_w = tw.ceil() as u32 + 4;
            let segment_h = th.ceil() as u32 + 4;
            if segment_w > 0 && segment_h > 0 {
                let mut seg = Canvas::new(segment_w, segment_h);
                seg.draw_text(&font, &text, 2.0, 2.0, text_scale, rgba);
                let rotated_seg = rotate_canvas(&seg, 180.0);

                let mut x = text_band_x_left as f32 + 4.0;
                let y = text_band_y_bot as f32;
                let x_limit = (inner_x + inner_w) as f32 - tw;
                while x < x_limit {
                    canvas.blit(&rotated_seg, x as i32, y as i32);
                    x += step;
                }
            }
        }

        // ── Left edge: text running top to bottom (rotated 90 clockwise = -90) ──
        {
            let segment_w = tw.ceil() as u32 + 4;
            let segment_h = th.ceil() as u32 + 4;
            if segment_w > 0 && segment_h > 0 {
                let mut seg = Canvas::new(segment_w, segment_h);
                seg.draw_text(&font, &text, 2.0, 2.0, text_scale, rgba);
                let rotated_seg = rotate_canvas(&seg, 90.0);

                let x = text_band_x_left as f32;
                let mut y = (text_band_y_top + th.ceil() as i32 + 8) as f32;
                let y_limit = (inner_y + inner_h) as f32 - tw;
                while y < y_limit {
                    canvas.blit(&rotated_seg, x as i32, y as i32);
                    y += step;
                }
            }
        }

        // ── Right edge: text running bottom to top (rotated -90 = 270) ──
        {
            let segment_w = tw.ceil() as u32 + 4;
            let segment_h = th.ceil() as u32 + 4;
            if segment_w > 0 && segment_h > 0 {
                let mut seg = Canvas::new(segment_w, segment_h);
                seg.draw_text(&font, &text, 2.0, 2.0, text_scale, rgba);
                let rotated_seg = rotate_canvas(&seg, -90.0);

                let x = text_band_x_right as f32;
                let mut y = (text_band_y_top + th.ceil() as i32 + 8) as f32;
                let y_limit = (inner_y + inner_h) as f32 - tw;
                while y < y_limit {
                    canvas.blit(&rotated_seg, x as i32, y as i32);
                    y += step;
                }
            }
        }

        // ── Corner decorations: ornamental cross/plus at each corner ──
        let mut rng = rand::thread_rng();
        // Random corner ornament size: ±2px
        let corner_size = (inner_gap as f32 * 0.5).ceil() as i32 + rng.gen_range(-2..=2);
        let corners = [
            (outer_x + outer_thickness as i32 / 2, outer_y + outer_thickness as i32 / 2),
            (outer_x + outer_w - outer_thickness as i32 / 2, outer_y + outer_thickness as i32 / 2),
            (outer_x + outer_thickness as i32 / 2, outer_y + outer_h - outer_thickness as i32 / 2),
            (outer_x + outer_w - outer_thickness as i32 / 2, outer_y + outer_h - outer_thickness as i32 / 2),
        ];

        for &(ccx, ccy) in &corners {
            // Draw a small ornamental cross (+) at each corner
            canvas.draw_thick_line(
                ccx - corner_size,
                ccy,
                ccx + corner_size,
                ccy,
                2,
                rgba,
            );
            canvas.draw_thick_line(
                ccx,
                ccy - corner_size,
                ccx,
                ccy + corner_size,
                2,
                rgba,
            );
            // Small filled square at the center of each cross
            let sq = (corner_size / 3).max(2);
            canvas.fill_rect(
                ccx - sq,
                ccy - sq,
                (sq * 2) as u32,
                (sq * 2) as u32,
                light_rgba,
            );
            // Small diamond at the very center
            let diamond = vec![
                (ccx, ccy - sq + 1),
                (ccx + sq - 1, ccy),
                (ccx, ccy + sq - 1),
                (ccx - sq + 1, ccy),
            ];
            canvas.fill_polygon(&diamond, rgba);
        }

        // ── Small decorative dots along the midpoint of each border edge ──
        let dot_r = 2;
        // Random dot spacing: ±15%
        let base_dot_spacing = (inner_gap as f32 * 0.8) as i32;
        let dot_spacing = (base_dot_spacing as f32 * rng.gen_range(0.85..1.15)).round() as i32;
        let mid_band = outer_thickness as i32 + inner_gap / 2;

        // Top and bottom midpoints: dots between the corner decorations
        {
            let y_top = outer_y + mid_band;
            let y_bot = outer_y + outer_h - mid_band;
            let x_start = outer_x + corner_size * 3;
            let x_end = outer_x + outer_w - corner_size * 3;
            let mut x = x_start;
            while x < x_end {
                canvas.fill_circle(x, y_top, dot_r, light_rgba);
                canvas.fill_circle(x, y_bot, dot_r, light_rgba);
                x += dot_spacing;
            }
        }
        // Left and right midpoints
        {
            let x_left = outer_x + mid_band;
            let x_right = outer_x + outer_w - mid_band;
            let y_start = outer_y + corner_size * 3;
            let y_end = outer_y + outer_h - corner_size * 3;
            let mut y = y_start;
            while y < y_end {
                canvas.fill_circle(x_left, y, dot_r, light_rgba);
                canvas.fill_circle(x_right, y, dot_r, light_rgba);
                y += dot_spacing;
            }
        }

        // ── Interior secondary text: intercalated main + secondary rows ──
        // Fill the area inside the inner border with subtle repeated text for
        // full-page security coverage, so cropping the border still leaves marks.
        if inner_w > 0 && inner_h > 0 {
            let interior_x = inner_x + 4;
            let interior_y = inner_y + 4;
            let interior_w = (inner_w - 8).max(0);
            let interior_h = (inner_h - 8).max(0);

            if interior_w > 0 && interior_h > 0 {
                let interior_font = load_font(config.font.as_deref(), FontWeight::Regular)?;
                let interior_scale = text_scale * 0.85;
                let sec_scale = text_scale * 0.55;

                // Random interior text opacity variation (25-35% of configured opacity)
                let interior_opacity = config.opacity * rng.gen_range(0.25..0.35);
                let interior_color = with_opacity(config.color, interior_opacity);
                let interior_rgba = to_rgba(interior_color);

                let (main_tw, main_th) = measure_text(&interior_font, &text, interior_scale);
                let has_secondary = !secondary_text.is_empty();
                let (sec_tw, _sec_th) = if has_secondary {
                    measure_text(&interior_font, &secondary_text, sec_scale)
                } else {
                    (0.0, 0.0)
                };

                let h_gap = 40.0_f32;
                let main_cell_w = main_tw + h_gap;
                let sec_cell_w = if has_secondary { sec_tw + h_gap } else { main_cell_w };
                let row_h = main_th + h_gap * 0.5;

                if main_cell_w > 0.0 && row_h > 0.0 {
                    let rows = (interior_h as f32 / row_h).ceil() as i32 + 1;
                    let main_cols = (interior_w as f32 / main_cell_w).ceil() as i32 + 2;
                    let sec_cols = (interior_w as f32 / sec_cell_w).ceil() as i32 + 2;

                    for row in 0..rows {
                        let is_secondary_row = has_secondary && row % 2 != 0;
                        let stagger = if row % 2 != 0 { main_cell_w / 2.0 } else { 0.0 };
                        let y = interior_y as f32 + row as f32 * row_h;

                        if y > (interior_y + interior_h) as f32 {
                            break;
                        }

                        if is_secondary_row {
                            for col in -1..sec_cols {
                                let x = interior_x as f32 + col as f32 * sec_cell_w + stagger;
                                if x > (interior_x + interior_w) as f32 {
                                    break;
                                }
                                canvas.draw_text(
                                    &interior_font,
                                    &secondary_text,
                                    x,
                                    y,
                                    sec_scale,
                                    interior_rgba,
                                );
                            }
                        } else {
                            for col in -1..main_cols {
                                let x = interior_x as f32 + col as f32 * main_cell_w + stagger;
                                if x > (interior_x + interior_w) as f32 {
                                    break;
                                }
                                canvas.draw_text(
                                    &interior_font,
                                    &text,
                                    x,
                                    y,
                                    interior_scale,
                                    interior_rgba,
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(canvas)
    }
}
