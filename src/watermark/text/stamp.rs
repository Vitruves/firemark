use rand::Rng;

use crate::cli::args::FontWeight;
use crate::config::types::WatermarkConfig;
use crate::error::Result;
use crate::render::canvas::Canvas;
use crate::render::color::{to_rgba, with_opacity};
use crate::render::font::{auto_scale, load_font, measure_text};
use crate::render::transform::rotate_canvas;
use crate::template::{self, TemplateContext};
use crate::watermark::background::render_text_background;
use crate::watermark::renderer::WatermarkRenderer;

/// Large prominent rubber-stamp watermark centred on the document.
///
/// Produces a clean double-bordered rectangle with bold uppercase text,
/// slightly tilted, with optional secondary text and full-page background.
pub struct StampRenderer;

impl WatermarkRenderer for StampRenderer {
    fn render(&self, config: &WatermarkConfig, width: u32, height: u32) -> Result<Canvas> {
        let ctx = TemplateContext::default();
        let text = template::resolve(&config.main_text, &ctx).to_uppercase();

        // Force bold weight for the stamp aesthetic.
        let weight = match config.font_weight {
            FontWeight::Thin | FontWeight::Light | FontWeight::Regular => FontWeight::Bold,
            other => other,
        };
        let font = load_font(config.font.as_deref(), weight)?;

        // The stamp text should span roughly 50-60% of the page width.
        let target_ratio = (config.scale * 1.4).clamp(0.40, 0.65);
        let scale = config.font_size.unwrap_or_else(|| {
            auto_scale(&text, width, target_ratio, &font)
        });

        let (tw, th) = measure_text(&font, &text, scale);
        let color = with_opacity(config.color, config.opacity);
        let rgba = to_rgba(color);

        // Generous padding around the text inside the borders.
        let pad_x = (scale * 0.55) as u32;
        let pad_y = (scale * 0.40) as u32;
        let mut rng = rand::thread_rng();
        let border_w = config.border_width.max(3);
        // Random border gap variation: +1..+4 instead of fixed +2
        let gap = border_w + rng.gen_range(1..4);
        // Random border line width variation: ±1px
        let border_w = (border_w as i32 + rng.gen_range(-1..=1)).max(2) as u32;

        let stamp_w = tw.ceil() as u32 + pad_x * 2 + (border_w + gap + border_w) * 2;
        let stamp_h = th.ceil() as u32 + pad_y * 2 + (border_w + gap + border_w) * 2;

        let mut stamp = Canvas::new(stamp_w, stamp_h);

        // --- Draw solid double border ---
        // Outer border: thick solid rectangle
        for i in 0..border_w {
            let offset = i as i32;
            stamp.draw_rect(offset, offset, stamp_w - i * 2, stamp_h - i * 2, rgba);
        }
        // Inner border: thinner solid rectangle inset by gap
        let inset = border_w + gap;
        for i in 0..border_w.max(2) {
            let offset = inset as i32 + i as i32;
            let iw = stamp_w.saturating_sub((inset + i) * 2);
            let ih = stamp_h.saturating_sub((inset + i) * 2);
            if iw > 0 && ih > 0 {
                stamp.draw_rect(offset, offset, iw, ih, rgba);
            }
        }

        // --- Centre the text inside the inner border ---
        let tx = (stamp_w as f32 - tw) / 2.0;
        let ty = (stamp_h as f32 - th) / 2.0;
        stamp.draw_text(&font, &text, tx, ty, scale, rgba);

        // --- Add secondary text below the main text (if non-empty) ---
        let secondary = template::resolve(&config.secondary_text, &ctx);
        if !secondary.is_empty() {
            let sec_scale = scale * 0.35;
            let (sw, _sh) = measure_text(&font, &secondary, sec_scale);
            let sx = (stamp_w as f32 - sw) / 2.0;
            let sy = ty + th + pad_y as f32 * 0.2;
            stamp.draw_text(&font, &secondary, sx, sy, sec_scale, rgba);
        }

        // --- Embed in square canvas before rotating (prevents clipping) ---
        let angle = if (config.rotation + 45.0).abs() < 0.5 {
            -15.0 // override default -45 for stamp
        } else {
            config.rotation
        };
        let diag = ((stamp_w as f32).powi(2) + (stamp_h as f32).powi(2))
            .sqrt()
            .ceil() as u32
            + 4;
        let mut padded = Canvas::new(diag, diag);
        let pad_ox = (diag as i32 - stamp_w as i32) / 2;
        let pad_oy = (diag as i32 - stamp_h as i32) / 2;
        padded.blit(&stamp, pad_ox, pad_oy);

        let rotated = rotate_canvas(&padded, angle);

        // Render subtle intercalated text background behind the stamp.
        let mut canvas = render_text_background(config, width, height, 0.3)?;

        // Blit the stamp onto the background canvas, centred with random offset ±3px.
        let ox = (width as i32 - rotated.width() as i32) / 2 + config.offset.0 + rng.gen_range(-3..=3);
        let oy = (height as i32 - rotated.height() as i32) / 2 + config.offset.1 + rng.gen_range(-3..=3);
        canvas.blit(&rotated, ox, oy);

        Ok(canvas)
    }
}
