use crate::cli::args::BlendMode;
use crate::render::canvas::Canvas;

/// A per-channel blend function: `(base, overlay) -> blended`.
pub type BlendFn = fn(u8, u8) -> u8;

/// Return the blend function corresponding to the given blend mode.
pub fn get_blend_fn(mode: BlendMode) -> BlendFn {
    match mode {
        BlendMode::Normal => blend_normal,
        BlendMode::Multiply => blend_multiply,
        BlendMode::Screen => blend_screen,
        BlendMode::Overlay => blend_overlay,
        BlendMode::SoftLight => blend_soft_light,
    }
}

/// Composite `overlay` onto `base` at pixel offset `(x, y)` with the given
/// opacity and blend mode.
pub fn composite(
    base: &mut Canvas,
    overlay: &Canvas,
    x: i32,
    y: i32,
    opacity: f32,
    mode: BlendMode,
) {
    let blend = get_blend_fn(mode);
    let opacity = opacity.clamp(0.0, 1.0);

    let base_w = base.width() as i32;
    let base_h = base.height() as i32;
    let overlay_img = overlay.image();

    for oy in 0..overlay.height() {
        for ox in 0..overlay.width() {
            let bx = x + ox as i32;
            let by = y + oy as i32;

            if bx < 0 || by < 0 || bx >= base_w || by >= base_h {
                continue;
            }

            let src = overlay_img.get_pixel(ox, oy);
            let src_a = (src[3] as f32 / 255.0) * opacity;
            if src_a <= 0.0 {
                continue;
            }

            let dst = base.pixel_mut(bx as u32, by as u32);

            let dst_a = dst[3] as f32 / 255.0;
            let out_a = src_a + dst_a * (1.0 - src_a);

            if out_a <= 0.0 {
                *dst = image::Rgba([0, 0, 0, 0]);
                continue;
            }

            let mut out = [0u8; 4];
            for ch in 0..3 {
                let blended = blend(dst[ch], src[ch]);
                let mixed = blended as f32 * src_a + dst[ch] as f32 * dst_a * (1.0 - src_a);
                out[ch] = (mixed / out_a).round().clamp(0.0, 255.0) as u8;
            }
            out[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;

            *dst = image::Rgba(out);
        }
    }
}

/// Composite `overlay` centered on `base` with the given opacity and blend mode.
pub fn composite_centered(base: &mut Canvas, overlay: &Canvas, opacity: f32, mode: BlendMode) {
    let x = (base.width() as i32 - overlay.width() as i32) / 2;
    let y = (base.height() as i32 - overlay.height() as i32) / 2;
    composite(base, overlay, x, y, opacity, mode);
}

// ── Blend functions ──

fn blend_normal(_base: u8, src: u8) -> u8 {
    src
}

fn blend_multiply(base: u8, src: u8) -> u8 {
    ((base as u16 * src as u16) / 255) as u8
}

fn blend_screen(base: u8, src: u8) -> u8 {
    (255 - ((255 - base as u16) * (255 - src as u16) / 255)) as u8
}

fn blend_overlay(base: u8, src: u8) -> u8 {
    if base < 128 {
        ((2 * base as u16 * src as u16) / 255) as u8
    } else {
        (255 - 2 * (255 - base as u16) * (255 - src as u16) / 255) as u8
    }
}

fn blend_soft_light(base: u8, src: u8) -> u8 {
    let b = base as f32 / 255.0;
    let s = src as f32 / 255.0;
    let result = if s <= 0.5 {
        b - (1.0 - 2.0 * s) * b * (1.0 - b)
    } else {
        let d = if b <= 0.25 {
            ((16.0 * b - 12.0) * b + 4.0) * b
        } else {
            b.sqrt()
        };
        b + (2.0 * s - 1.0) * (d - b)
    };
    (result.clamp(0.0, 1.0) * 255.0).round() as u8
}
