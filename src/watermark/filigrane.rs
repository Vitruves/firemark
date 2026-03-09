use std::f32::consts::PI;

use image::Rgba;
use rand::Rng;

use crate::cli::args::FiligraneStyle;
use crate::render::canvas::Canvas;

/// Render cryptographic filigrane security patterns.
///
/// Produces complex geometric patterns inspired by banknote security features:
/// guilloche wave envelopes, spirograph rosettes, fine crosshatch grids,
/// Lissajous figures, moiré interference, spirals, honeycomb meshes,
/// and decorative wavy borders.
pub fn render_filigrane(
    width: u32,
    height: u32,
    base_color: [u8; 4],
    opacity: f32,
    style: FiligraneStyle,
) -> Canvas {
    let mut canvas = Canvas::new(width, height);
    let w = width as f32;
    let h = height as f32;
    let dim = w.min(h);

    if dim < 80.0 || style == FiligraneStyle::None {
        return canvas;
    }

    let color = make_color(base_color, opacity);
    let faint = make_color(base_color, opacity * 0.45);

    match style {
        FiligraneStyle::Full => {
            draw_guilloche_bands(&mut canvas, w, h, dim, color);
            draw_spirograph_rosette(&mut canvas, w, h, dim, color);
            draw_corner_rosettes(&mut canvas, w, h, dim, faint);
            draw_crosshatch(&mut canvas, w, h, dim, faint);
            draw_security_border(&mut canvas, w, h, dim, color);
        }
        FiligraneStyle::Guilloche => {
            draw_guilloche_bands(&mut canvas, w, h, dim, color);
        }
        FiligraneStyle::Rosette => {
            draw_spirograph_rosette(&mut canvas, w, h, dim, color);
            draw_corner_rosettes(&mut canvas, w, h, dim, faint);
        }
        FiligraneStyle::Crosshatch => {
            draw_crosshatch(&mut canvas, w, h, dim, faint);
        }
        FiligraneStyle::Border => {
            draw_security_border(&mut canvas, w, h, dim, color);
        }
        FiligraneStyle::Lissajous => {
            draw_lissajous(&mut canvas, w, h, dim, color);
        }
        FiligraneStyle::Moire => {
            draw_moire(&mut canvas, w, h, dim, color);
        }
        FiligraneStyle::Spiral => {
            draw_spiral(&mut canvas, w, h, dim, color);
        }
        FiligraneStyle::Mesh => {
            draw_mesh(&mut canvas, w, h, dim, faint);
        }
        FiligraneStyle::None => {}
    }

    canvas
}

fn make_color(base: [u8; 4], opacity: f32) -> Rgba<u8> {
    Rgba([
        base[0],
        base[1],
        base[2],
        (base[3] as f32 * opacity).clamp(0.0, 255.0) as u8,
    ])
}

// ── Guilloche wave envelope bands ────────────────────────────────────────────

fn draw_guilloche_bands(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let num_bands = ((h / dim) * 5.0).ceil().max(3.0).min(7.0) as usize;
    let band_spacing = h / (num_bands as f32 + 1.0);

    for band in 1..=num_bands {
        let cy = band as f32 * band_spacing;
        let amplitude = band_spacing * 0.18;
        // Randomize frequencies
        let freq_fast = rng.gen_range(7.5..8.5) * PI / w;
        let freq_slow = rng.gen_range(1.8..2.2) * PI / w;
        // Random phase per band
        let band_phase: f32 = rng.gen_range(0.0..2.0 * PI);

        let num_lines = 16;
        for line in 0..num_lines {
            let phase = line as f32 * PI / num_lines as f32 + band_phase;
            let y_spread = (line as f32 - num_lines as f32 / 2.0) * 0.7;

            let mut x = 0.0_f32;
            while x < w {
                let y = cy
                    + y_spread
                    + amplitude
                        * (freq_fast * x + phase).sin()
                        * (freq_slow * x + phase * 0.3).cos();
                canvas.blend_pixel(x as i32, y as i32, color);
                x += 1.0;
            }
        }
    }
}

// ── Spirograph rosette ───────────────────────────────────────────────────────

fn draw_spirograph_rosette(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let cx = w / 2.0;
    let cy = h / 2.0;
    let base = dim * 0.22;

    // Random starting angle
    let start_angle: f32 = rng.gen_range(0.0..2.0 * PI);

    let patterns: &[(f32, f32, f32)] = &[
        (base, base * 0.40, base * 0.30),
        (base * 0.80, base * 0.20, base * 0.22),
        (base * 0.60, base * 0.15, base * 0.17),
    ];

    let max_t = 40.0 * PI;
    let steps = 12_000_u32;

    for &(big_r, small_r, d) in patterns {
        // Vary small_r/big_r ratio by ±0.02
        let ratio_jitter: f32 = rng.gen_range(-0.02..0.02);
        let ratio = (big_r - small_r) / small_r + ratio_jitter;
        for i in 0..steps {
            let t = i as f32 / steps as f32 * max_t + start_angle;
            let x = cx + (big_r - small_r) * t.cos() + d * (ratio * t).cos();
            let y = cy + (big_r - small_r) * t.sin() - d * (ratio * t).sin();
            canvas.blend_pixel(x as i32, y as i32, color);
        }
    }

    // Concentric modulated circles for moiré depth.
    let num_rings = 8;
    for ring in 1..=num_rings {
        let r = base * 0.08 * ring as f32;
        let petals = 6 + ring * 3;
        let modulation = r * 0.12;
        let ring_steps = 720_u32;
        for i in 0..ring_steps {
            let theta = i as f32 * 2.0 * PI / ring_steps as f32 + start_angle;
            let rr = r + modulation * (petals as f32 * theta).sin();
            let x = cx + rr * theta.cos();
            let y = cy + rr * theta.sin();
            canvas.blend_pixel(x as i32, y as i32, color);
        }
    }
}

// ── Corner rose curves ───────────────────────────────────────────────────────

fn draw_corner_rosettes(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let r = dim * 0.09;
    let margin = dim * 0.10;

    let corners = [
        (margin, margin),
        (w - margin, margin),
        (margin, h - margin),
        (w - margin, h - margin),
    ];

    for (cx, cy) in corners {
        for &(k, scale) in &[(5.0_f32, 1.0_f32), (3.0, 0.6)] {
            let rr = r * scale;
            let steps = 1200_u32;
            for i in 0..steps {
                let theta = i as f32 / steps as f32 * 2.0 * PI;
                let radius = rr * (k * theta).cos();
                let x = cx + radius * theta.cos();
                let y = cy + radius * theta.sin();
                canvas.blend_pixel(x as i32, y as i32, color);
            }
        }
    }
}

// ── Diamond crosshatch grid ──────────────────────────────────────────────────

fn draw_crosshatch(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let base_spacing = (dim * 0.05).max(18.0);
    // Vary spacing ±10%
    let spacing = base_spacing * rng.gen_range(0.90..1.10);
    // Random phase offset per line set
    let phase1: f32 = rng.gen_range(0.0..spacing);
    let phase2: f32 = rng.gen_range(0.0..spacing);

    let reach = w + h;
    let num_lines = (reach / spacing) as i32 + 1;

    for i in (-num_lines)..=num_lines {
        let offset = i as f32 * spacing + phase1;
        canvas.draw_line(offset as i32, 0, (offset + h) as i32, h as i32, color);
    }
    for i in (-num_lines)..=num_lines {
        let offset = i as f32 * spacing + phase2;
        canvas.draw_line(
            (w - offset) as i32,
            0,
            (w - offset - h) as i32,
            h as i32,
            color,
        );
    }
}

// ── Wavy security border ─────────────────────────────────────────────────────

fn draw_security_border(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let margin = (dim * 0.025).max(8.0);
    let base_amplitude = (dim * 0.006).max(2.0);
    let freq = 12.0 * PI / dim;

    for ring in 0..3u32 {
        let m = margin + ring as f32 * 3.5;
        // Random phase per ring
        let phase: f32 = ring as f32 * 1.2 + rng.gen_range(0.0..PI);
        // Vary amplitude ±10%
        let amplitude = base_amplitude * rng.gen_range(0.90..1.10);

        let mut x = 0.0_f32;
        while x < w {
            let dy = amplitude * (freq * x + phase).sin();
            canvas.blend_pixel(x as i32, (m + dy) as i32, color);
            canvas.blend_pixel(x as i32, (h - m + dy) as i32, color);
            x += 1.0;
        }

        let mut y = 0.0_f32;
        while y < h {
            let dx = amplitude * (freq * y + phase).sin();
            canvas.blend_pixel((m + dx) as i32, y as i32, color);
            canvas.blend_pixel((w - m + dx) as i32, y as i32, color);
            y += 1.0;
        }
    }
}

// ── Lissajous figures ────────────────────────────────────────────────────────

fn draw_lissajous(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let cx = w / 2.0;
    let cy = h / 2.0;

    let figures: &[(f32, f32, f32, f32, f32)] = &[
        (3.0, 2.0, PI / 4.0, 0.40, 0.40),
        (5.0, 4.0, PI / 6.0, 0.30, 0.30),
        (7.0, 6.0, PI / 3.0, 0.22, 0.22),
        (3.0, 4.0, PI / 2.0, 0.35, 0.25),
        (5.0, 6.0, PI / 5.0, 0.18, 0.18),
    ];

    let steps = 8_000_u32;
    let max_t = 2.0 * PI;

    for &(a, b, delta, sx, sy) in figures {
        let ax = dim * sx;
        let ay = dim * sy;
        for i in 0..steps {
            let t = i as f32 / steps as f32 * max_t;
            let x = cx + ax * (a * t + delta).sin();
            let y = cy + ay * (b * t).sin();
            canvas.blend_pixel(x as i32, y as i32, color);
        }
    }
}

// ── Moiré interference ───────────────────────────────────────────────────────

fn draw_moire(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Randomize radial frequency ±5%
    let base_spacing = (dim * 0.015).max(6.0);
    let spacing = base_spacing * rng.gen_range(0.95..1.05);
    let max_r = ((w * w + h * h).sqrt() / 2.0) as u32;

    // Vary center offsets by ±15%
    let base_offset = dim * 0.08;
    let offset1 = base_offset * rng.gen_range(0.85..1.15);
    let offset2 = base_offset * rng.gen_range(0.85..1.15);
    let centres = [
        (w / 2.0 - offset1, h / 2.0 - offset1),
        (w / 2.0 + offset2, h / 2.0 + offset2),
    ];

    for &(cx, cy) in &centres {
        let mut r = spacing;
        while r < max_r as f32 {
            let steps = (2.0 * PI * r).ceil().max(120.0) as u32;
            for i in 0..steps {
                let theta = i as f32 * 2.0 * PI / steps as f32;
                let x = cx + r * theta.cos();
                let y = cy + r * theta.sin();
                canvas.blend_pixel(x as i32, y as i32, color);
            }
            r += spacing;
        }
    }
}

// ── Archimedean spiral ───────────────────────────────────────────────────────

fn draw_spiral(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let cx = w / 2.0;
    let cy = h / 2.0;
    let max_r = (w * w + h * h).sqrt() / 2.0;
    // Vary arm spacing ±10%
    let base_arm_spacing = (dim * 0.025).max(8.0);
    let arm_spacing = base_arm_spacing * rng.gen_range(0.90..1.10);

    // Randomize starting angle
    let start_angle: f32 = rng.gen_range(0.0..2.0 * PI);

    let num_arms = 6;
    let steps = 20_000_u32;
    let max_theta = max_r / arm_spacing * 2.0 * PI;

    for arm in 0..num_arms {
        let phase = arm as f32 * 2.0 * PI / num_arms as f32 + start_angle;
        for i in 0..steps {
            let theta = i as f32 / steps as f32 * max_theta + phase;
            let r = arm_spacing * theta / (2.0 * PI);
            if r > max_r {
                break;
            }
            let x = cx + r * theta.cos();
            let y = cy + r * theta.sin();
            canvas.blend_pixel(x as i32, y as i32, color);
        }
    }
}

// ── Hexagonal honeycomb mesh ─────────────────────────────────────────────────

fn draw_mesh(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let base_cell_r = (dim * 0.03).max(12.0);
    // Vary cell radius ±8%
    let cell_r = base_cell_r * rng.gen_range(0.92..1.08);
    // Random hex grid rotation (0-30 deg)
    let grid_rotation: f32 = rng.gen_range(0.0..30.0_f32).to_radians();

    let hex_w = cell_r * 3.0_f32.sqrt();
    let hex_h = cell_r * 2.0;

    let cols = (w / hex_w) as i32 + 3;
    let rows = (h / (hex_h * 0.75)) as i32 + 3;

    let center_x = w / 2.0;
    let center_y = h / 2.0;

    for row in -2..rows {
        let y_off = row as f32 * hex_h * 0.75;
        let x_stagger = if row % 2 != 0 { hex_w / 2.0 } else { 0.0 };

        for col in -2..cols {
            let raw_cx = col as f32 * hex_w + x_stagger;
            let raw_cy = y_off;
            // Apply grid rotation around center
            let dx = raw_cx - center_x;
            let dy = raw_cy - center_y;
            let rot_cx = center_x + dx * grid_rotation.cos() - dy * grid_rotation.sin();
            let rot_cy = center_y + dx * grid_rotation.sin() + dy * grid_rotation.cos();
            draw_hexagon(canvas, rot_cx, rot_cy, cell_r, color);
        }
    }
}

fn draw_hexagon(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Rgba<u8>) {
    let mut pts = [(0i32, 0i32); 6];
    for i in 0..6 {
        let angle = PI / 6.0 + i as f32 * PI / 3.0;
        pts[i] = (
            (cx + r * angle.cos()) as i32,
            (cy + r * angle.sin()) as i32,
        );
    }
    for i in 0..6 {
        let j = (i + 1) % 6;
        canvas.draw_line(pts[i].0, pts[i].1, pts[j].0, pts[j].1, color);
    }
}
