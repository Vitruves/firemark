use std::f32::consts::TAU;

use image::RgbaImage;
use rand::Rng;

use crate::render::color::{adaptive_ink, luminance};
use crate::render::saliency::{row_gradient_density, smooth_density, SaliencyMap};

/// Draw thin, wavy, low-opacity ink strokes through the document's densest
/// text bands and most salient regions.
///
/// The strokes partially occlude real glyph strokes and figure edges at an
/// opacity low enough to keep the document readable. An AI inpainting model
/// that removes them must reconstruct the occluded content it cannot know,
/// so removal becomes visibly lossy instead of clean — the core deterrent
/// against ChatGPT-style watermark removal.
pub fn entangle_strokes(base: &mut RgbaImage, color: [u8; 4], opacity: f32) {
    let (w, h) = (base.width(), base.height());
    if w < 200 || h < 200 {
        return;
    }
    let mut rng = rand::thread_rng();

    let alpha = (opacity.clamp(0.0, 1.0) * 90.0).clamp(18.0, 64.0) as u8;
    let ink = [color[0], color[1], color[2], alpha];

    // ── Horizontal strokes through the densest text bands ──
    let kernel = (h as usize / 60).max(3);
    let density = smooth_density(&row_gradient_density(base), kernel);
    let bands = top_bands(&density, 8, (h / 24).max(8) as usize);
    for y_band in bands {
        let angle = rng.gen_range(-1.5f32..1.5).to_radians();
        let amp = (h as f32 * 0.004).clamp(1.5, 6.0);
        draw_wavy_stroke(base, &mut rng, ink, (0.0, y_band as f32), angle, w as f32 * 1.05, amp);
    }

    // ── Diagonal strokes through salient regions ──
    let saliency = SaliencyMap::from_image(base);
    let dim = w.min(h) as f32;
    let mut drawn = 0;
    for _ in 0..60 {
        if drawn >= 4 {
            break;
        }
        let px = rng.gen_range(0..w) as i32;
        let py = rng.gen_range(0..h) as i32;
        if saliency.score_at(px, py) < 0.35 {
            continue;
        }
        let base_angle = rng.gen_range(20.0f32..70.0);
        let angle = if rng.gen::<bool>() { base_angle } else { 180.0 - base_angle }.to_radians();
        let length = dim * rng.gen_range(0.6..1.2);
        let start = (
            px as f32 - angle.cos() * length / 2.0,
            py as f32 - angle.sin() * length / 2.0,
        );
        let amp = (h as f32 * 0.008).clamp(2.0, 12.0);
        draw_wavy_stroke(base, &mut rng, ink, start, angle, length, amp);
        drawn += 1;
    }
}

/// Indices of the `count` highest-density rows, at least `min_gap` apart.
fn top_bands(density: &[f64], count: usize, min_gap: usize) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..density.len()).collect();
    idx.sort_by(|&a, &b| density[b].partial_cmp(&density[a]).unwrap_or(std::cmp::Ordering::Equal));

    let mut out: Vec<usize> = Vec::with_capacity(count);
    for i in idx {
        if out.len() >= count {
            break;
        }
        if out.iter().all(|&s| (s as i64 - i as i64).unsigned_abs() as usize >= min_gap) {
            out.push(i);
        }
    }
    out
}

/// Draw an anti-aliased ~1px stroke along a wavy path: a straight carrier at
/// `angle` from `start`, displaced along its normal by two randomly-phased
/// sines. Ink adapts per-pixel to the local background luminance.
fn draw_wavy_stroke(
    img: &mut RgbaImage,
    rng: &mut impl Rng,
    ink: [u8; 4],
    start: (f32, f32),
    angle: f32,
    length: f32,
    amp: f32,
) {
    let (dx, dy) = (angle.cos(), angle.sin());
    let (nx, ny) = (-dy, dx);
    let wl1 = length * rng.gen_range(0.15..0.4);
    let wl2 = wl1 * rng.gen_range(0.23..0.41);
    let p1 = rng.gen_range(0.0..TAU);
    let p2 = rng.gen_range(0.0..TAU);

    let steps = length.max(1.0) as i32;
    for i in 0..steps {
        let t = i as f32;
        let off = amp * ((t / wl1 * TAU + p1).sin() + 0.4 * (t / wl2 * TAU + p2).sin());
        let x = start.0 + dx * t + nx * off;
        let y = start.1 + dy * t + ny * off;
        plot_soft(img, x, y, ink);
    }
}

/// Deposit ink at a fractional position, distributing alpha over the 2×2
/// pixel neighborhood by coverage (anti-aliasing).
fn plot_soft(img: &mut RgbaImage, x: f32, y: f32, ink: [u8; 4]) {
    let (w, h) = (img.width() as i32, img.height() as i32);
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = x - x0;
    let fy = y - y0;

    let taps = [
        (x0 as i32, y0 as i32, (1.0 - fx) * (1.0 - fy)),
        (x0 as i32 + 1, y0 as i32, fx * (1.0 - fy)),
        (x0 as i32, y0 as i32 + 1, (1.0 - fx) * fy),
        (x0 as i32 + 1, y0 as i32 + 1, fx * fy),
    ];

    for (px, py, weight) in taps {
        if px < 0 || py < 0 || px >= w || py >= h || weight <= 0.0 {
            continue;
        }
        let bg = *img.get_pixel(px as u32, py as u32);
        let color = adaptive_ink(ink, luminance(&bg));
        let a = ink[3] as f32 / 255.0 * weight;
        let inv = 1.0 - a;
        let blended = image::Rgba([
            (color[0] as f32 * a + bg.0[0] as f32 * inv).round() as u8,
            (color[1] as f32 * a + bg.0[1] as f32 * inv).round() as u8,
            (color[2] as f32 * a + bg.0[2] as f32 * inv).round() as u8,
            (bg.0[3] as f32 + ink[3] as f32 * weight * inv).min(255.0).round() as u8,
        ]);
        img.put_pixel(px as u32, py as u32, blended);
    }
}
