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

    let color = make_color(base_color, opacity * 1.8);
    let faint = make_color(base_color, opacity * 1.0);

    match style {
        FiligraneStyle::Full => {
            // Stack all geometric/structured patterns that layer well.
            // Whole-surface organic patterns (plume, constellation, ripple)
            // are excluded — they are designed to stand alone.
            draw_guilloche_bands(&mut canvas, w, h, dim, color);
            draw_spirograph_rosette(&mut canvas, w, h, dim, color);
            draw_corner_rosettes(&mut canvas, w, h, dim, faint);
            draw_crosshatch(&mut canvas, w, h, dim, faint);
            draw_security_border(&mut canvas, w, h, dim, color);
            draw_lissajous(&mut canvas, w, h, dim, faint);
            draw_moire(&mut canvas, w, h, dim, faint);
            draw_spiral(&mut canvas, w, h, dim, faint);
            draw_mesh(&mut canvas, w, h, dim, faint);
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
        FiligraneStyle::Plume => {
            let strong = make_color(base_color, opacity * 1.8);
            draw_plume(&mut canvas, w, h, strong);
        }
        FiligraneStyle::Constellation => {
            let strong = make_color(base_color, opacity * 1.8);
            let medium = make_color(base_color, opacity * 1.2);
            draw_constellation(&mut canvas, w, h, strong, medium);
        }
        FiligraneStyle::Ripple => {
            let strong = make_color(base_color, opacity * 1.8);
            draw_ripple(&mut canvas, w, h, strong);
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
    let num_bands = rng
        .gen_range(3..=8_usize)
        .max(((h / dim) * 5.0).ceil() as usize);
    let base_spacing = h / (num_bands as f32 + 1.0);

    for band in 1..=num_bands {
        // Jitter band vertical position ±15%
        let cy =
            band as f32 * base_spacing + rng.gen_range(-base_spacing * 0.15..base_spacing * 0.15);
        let amplitude = base_spacing * rng.gen_range(0.14..0.24);
        // Heavily randomized frequencies per band
        let freq_fast = rng.gen_range(5.0..12.0) * PI / w;
        let freq_slow = rng.gen_range(1.2..3.0) * PI / w;
        let band_phase: f32 = rng.gen_range(0.0..2.0 * PI);
        // Third harmonic for more complex wave shape
        let freq_third = rng.gen_range(12.0..20.0) * PI / w;
        let third_amp = amplitude * rng.gen_range(0.05..0.2);

        let num_lines = rng.gen_range(12..20_u32);
        for line in 0..num_lines {
            let phase = line as f32 * PI / num_lines as f32 + band_phase;
            let y_spread = (line as f32 - num_lines as f32 / 2.0) * rng.gen_range(0.5..0.9);
            // Per-line frequency wobble
            let line_freq_jitter: f32 = rng.gen_range(0.95..1.05);

            let mut x = 0.0_f32;
            while x < w {
                let y = cy
                    + y_spread
                    + amplitude
                        * (freq_fast * line_freq_jitter * x + phase).sin()
                        * (freq_slow * x + phase * 0.3).cos()
                    + third_amp * (freq_third * x + phase * 1.7).sin();
                canvas.blend_pixel(x as i32, y as i32, color);
                // Double-draw for visibility
                canvas.blend_pixel(x as i32, y as i32 + 1, color);
                x += 1.0;
            }
        }
    }
}

// ── Spirograph rosette ───────────────────────────────────────────────────────

fn draw_spirograph_rosette(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Jitter center position ±5% of dim
    let cx = w / 2.0 + rng.gen_range(-dim * 0.05..dim * 0.05);
    let cy = h / 2.0 + rng.gen_range(-dim * 0.05..dim * 0.05);
    let base = dim * rng.gen_range(0.18..0.26);

    let start_angle: f32 = rng.gen_range(0.0..2.0 * PI);

    // Generate 3-5 spirograph layers with random parameters
    let num_patterns = rng.gen_range(3..=5_usize);
    let max_t = rng.gen_range(35.0..50.0) * PI;
    let steps = 14_000_u32;

    for p in 0..num_patterns {
        let scale = 1.0 - p as f32 * rng.gen_range(0.15..0.22);
        let big_r = base * scale;
        let small_r = big_r * rng.gen_range(0.15..0.45);
        let d = big_r * rng.gen_range(0.20..0.40);
        let ratio_jitter: f32 = rng.gen_range(-0.06..0.06);
        let ratio = (big_r - small_r) / small_r + ratio_jitter;
        let phase_offset: f32 = rng.gen_range(0.0..2.0 * PI);

        for i in 0..steps {
            let t = i as f32 / steps as f32 * max_t + start_angle + phase_offset;
            let x = cx + (big_r - small_r) * t.cos() + d * (ratio * t).cos();
            let y = cy + (big_r - small_r) * t.sin() - d * (ratio * t).sin();
            canvas.blend_pixel(x as i32, y as i32, color);
            canvas.blend_pixel(x as i32 + 1, y as i32, color);
        }
    }

    // Concentric modulated circles with more variation
    let num_rings = rng.gen_range(6..12_u32);
    for ring in 1..=num_rings {
        let r = base * rng.gen_range(0.06..0.10) * ring as f32;
        let petals = rng.gen_range(4..20_u32) + ring * 2;
        let modulation = r * rng.gen_range(0.08..0.18);
        let ring_steps = 900_u32;
        let ring_phase: f32 = rng.gen_range(0.0..2.0 * PI);
        for i in 0..ring_steps {
            let theta = i as f32 * 2.0 * PI / ring_steps as f32 + start_angle + ring_phase;
            let rr = r + modulation * (petals as f32 * theta).sin();
            let x = cx + rr * theta.cos();
            let y = cy + rr * theta.sin();
            canvas.blend_pixel(x as i32, y as i32, color);
        }
    }
}

// ── Corner rose curves ───────────────────────────────────────────────────────

fn draw_corner_rosettes(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let r = dim * rng.gen_range(0.07..0.12);
    let margin = dim * rng.gen_range(0.08..0.13);

    let corners = [
        (margin, margin),
        (w - margin, margin),
        (margin, h - margin),
        (w - margin, h - margin),
    ];

    for (cx, cy) in corners {
        // Jitter corner position
        let jx = cx + rng.gen_range(-dim * 0.02..dim * 0.02);
        let jy = cy + rng.gen_range(-dim * 0.02..dim * 0.02);
        // Random number of petal layers (2-4) with random k values
        let num_layers = rng.gen_range(2..=4_usize);
        for layer in 0..num_layers {
            let k = rng.gen_range(3.0..9.0_f32);
            let scale = 1.0 - layer as f32 * rng.gen_range(0.2..0.35);
            let rr = r * scale;
            let phase: f32 = rng.gen_range(0.0..2.0 * PI);
            let steps = 1500_u32;
            for i in 0..steps {
                let theta = i as f32 / steps as f32 * 2.0 * PI + phase;
                let radius = rr * (k * theta).cos();
                let x = jx + radius * theta.cos();
                let y = jy + radius * theta.sin();
                canvas.blend_pixel(x as i32, y as i32, color);
                canvas.blend_pixel(x as i32, y as i32 + 1, color);
            }
        }
    }
}

// ── Diamond crosshatch grid ──────────────────────────────────────────────────

fn draw_crosshatch(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let base_spacing = (dim * 0.05).max(18.0);
    // Vary global spacing ±20%
    let spacing = base_spacing * rng.gen_range(0.80..1.20);
    let phase1: f32 = rng.gen_range(0.0..spacing);
    let phase2: f32 = rng.gen_range(0.0..spacing);
    // Random angle offset so the grid isn't always exactly 45°
    let angle_jitter: f32 = rng.gen_range(-8.0..8.0_f32).to_radians();

    let reach = w + h;
    let num_lines = (reach / spacing) as i32 + 1;

    for i in (-num_lines)..=num_lines {
        // Per-line spacing jitter
        let line_jitter = rng.gen_range(-spacing * 0.08..spacing * 0.08);
        let offset = i as f32 * spacing + phase1 + line_jitter;
        let x_end = offset + h * (1.0 + angle_jitter.tan());
        canvas.draw_line(offset as i32, 0, x_end as i32, h as i32, color);
    }
    for i in (-num_lines)..=num_lines {
        let line_jitter = rng.gen_range(-spacing * 0.08..spacing * 0.08);
        let offset = i as f32 * spacing + phase2 + line_jitter;
        let x_start = w - offset;
        let x_end = x_start - h * (1.0 + angle_jitter.tan());
        canvas.draw_line(x_start as i32, 0, x_end as i32, h as i32, color);
    }
}

// ── Wavy security border ─────────────────────────────────────────────────────

fn draw_security_border(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let margin = (dim * rng.gen_range(0.02..0.035)).max(8.0);
    let base_amplitude = (dim * rng.gen_range(0.005..0.010)).max(2.0);
    let base_freq = rng.gen_range(8.0..16.0) * PI / dim;

    let num_rings = rng.gen_range(3..=6_u32);
    let ring_gap = rng.gen_range(2.5..5.0_f32);

    for ring in 0..num_rings {
        let m = margin + ring as f32 * ring_gap;
        let phase: f32 = rng.gen_range(0.0..2.0 * PI);
        let amplitude = base_amplitude * rng.gen_range(0.7..1.4);
        // Per-ring frequency variation
        let freq = base_freq * rng.gen_range(0.8..1.3);
        // Second harmonic for more complex wave
        let freq2 = freq * rng.gen_range(2.5..4.0);
        let amp2 = amplitude * rng.gen_range(0.1..0.3);

        let mut x = 0.0_f32;
        while x < w {
            let dy = amplitude * (freq * x + phase).sin() + amp2 * (freq2 * x + phase * 1.5).sin();
            canvas.blend_pixel(x as i32, (m + dy) as i32, color);
            canvas.blend_pixel(x as i32, (m + dy) as i32 + 1, color);
            canvas.blend_pixel(x as i32, (h - m + dy) as i32, color);
            canvas.blend_pixel(x as i32, (h - m + dy) as i32 + 1, color);
            x += 1.0;
        }

        let mut y = 0.0_f32;
        while y < h {
            let dx = amplitude * (freq * y + phase).sin() + amp2 * (freq2 * y + phase * 1.5).sin();
            canvas.blend_pixel((m + dx) as i32, y as i32, color);
            canvas.blend_pixel((m + dx) as i32 + 1, y as i32, color);
            canvas.blend_pixel((w - m + dx) as i32, y as i32, color);
            canvas.blend_pixel((w - m + dx) as i32 + 1, y as i32, color);
            y += 1.0;
        }
    }
}

// ── Lissajous figures ────────────────────────────────────────────────────────

fn draw_lissajous(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Jitter center
    let cx = w / 2.0 + rng.gen_range(-dim * 0.04..dim * 0.04);
    let cy = h / 2.0 + rng.gen_range(-dim * 0.04..dim * 0.04);

    // Generate 4-7 random Lissajous figures instead of fixed parameters
    let num_figures = rng.gen_range(4..=7_usize);
    let steps = 10_000_u32;
    let max_t = 2.0 * PI;

    for _ in 0..num_figures {
        let a = rng.gen_range(2.0..9.0_f32);
        let b = rng.gen_range(2.0..9.0_f32);
        let delta: f32 = rng.gen_range(0.0..2.0 * PI);
        let sx = rng.gen_range(0.15..0.45);
        let sy = rng.gen_range(0.15..0.45);
        let ax = dim * sx;
        let ay = dim * sy;
        let phase_offset: f32 = rng.gen_range(0.0..2.0 * PI);

        for i in 0..steps {
            let t = i as f32 / steps as f32 * max_t + phase_offset;
            let x = cx + ax * (a * t + delta).sin();
            let y = cy + ay * (b * t).sin();
            canvas.blend_pixel(x as i32, y as i32, color);
            canvas.blend_pixel(x as i32 + 1, y as i32, color);
        }
    }
}

// ── Moiré interference ───────────────────────────────────────────────────────

fn draw_moire(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let base_spacing = (dim * rng.gen_range(0.012..0.020)).max(6.0);
    let max_r = ((w * w + h * h).sqrt() / 2.0) as u32;

    // 2-4 centres with heavily randomized positions
    let num_centres = rng.gen_range(2..=4_usize);
    let mut centres = Vec::with_capacity(num_centres);
    for _ in 0..num_centres {
        let cx = w * rng.gen_range(0.2..0.8);
        let cy = h * rng.gen_range(0.2..0.8);
        centres.push((cx, cy));
    }

    for &(cx, cy) in &centres {
        // Per-centre spacing variation
        let spacing = base_spacing * rng.gen_range(0.8..1.3);
        let start_r = spacing * rng.gen_range(0.5..1.5);
        // Slight elliptical distortion per centre
        let stretch_x: f32 = rng.gen_range(0.85..1.15);
        let stretch_y: f32 = rng.gen_range(0.85..1.15);
        let wobble_freq = rng.gen_range(0.0..5.0_f32);
        let wobble_amp = spacing * rng.gen_range(0.0..0.15);

        let mut r = start_r;
        while r < max_r as f32 {
            let steps = (2.0 * PI * r).ceil().max(150.0) as u32;
            for i in 0..steps {
                let theta = i as f32 * 2.0 * PI / steps as f32;
                let rr = r + wobble_amp * (wobble_freq * theta).sin();
                let x = cx + rr * stretch_x * theta.cos();
                let y = cy + rr * stretch_y * theta.sin();
                canvas.blend_pixel(x as i32, y as i32, color);
            }
            // Slightly irregular ring spacing
            r += spacing * rng.gen_range(0.85..1.15);
        }
    }
}

// ── Archimedean spiral ───────────────────────────────────────────────────────

fn draw_spiral(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Jitter center
    let cx = w / 2.0 + rng.gen_range(-dim * 0.06..dim * 0.06);
    let cy = h / 2.0 + rng.gen_range(-dim * 0.06..dim * 0.06);
    let max_r = (w * w + h * h).sqrt() / 2.0;
    let base_arm_spacing = (dim * rng.gen_range(0.020..0.032)).max(8.0);

    let start_angle: f32 = rng.gen_range(0.0..2.0 * PI);

    let num_arms = rng.gen_range(4..=8_u32);
    let steps = 24_000_u32;

    for arm in 0..num_arms {
        // Per-arm spacing variation
        let arm_spacing = base_arm_spacing * rng.gen_range(0.85..1.15);
        let max_theta = max_r / arm_spacing * 2.0 * PI;
        let phase = arm as f32 * 2.0 * PI / num_arms as f32 + start_angle;
        // Per-arm wobble
        let wobble_freq = rng.gen_range(3.0..12.0);
        let wobble_amp = arm_spacing * rng.gen_range(0.1..0.3);

        for i in 0..steps {
            let theta = i as f32 / steps as f32 * max_theta + phase;
            let base_r = arm_spacing * theta / (2.0 * PI);
            let r = base_r + wobble_amp * (wobble_freq * theta).sin();
            if base_r > max_r {
                break;
            }
            let x = cx + r * theta.cos();
            let y = cy + r * theta.sin();
            canvas.blend_pixel(x as i32, y as i32, color);
            canvas.blend_pixel(x as i32, y as i32 + 1, color);
        }
    }
}

// ── Hexagonal honeycomb mesh ─────────────────────────────────────────────────

fn draw_mesh(canvas: &mut Canvas, w: f32, h: f32, dim: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    let base_cell_r = (dim * rng.gen_range(0.025..0.038)).max(12.0);
    // Random hex grid rotation (0-60 deg for full range)
    let grid_rotation: f32 = rng.gen_range(0.0..60.0_f32).to_radians();

    let hex_w = base_cell_r * 3.0_f32.sqrt();
    let hex_h = base_cell_r * 2.0;

    let cols = (w / hex_w) as i32 + 4;
    let rows = (h / (hex_h * 0.75)) as i32 + 4;

    let center_x = w / 2.0;
    let center_y = h / 2.0;

    for row in -3..rows {
        let y_off = row as f32 * hex_h * 0.75;
        let x_stagger = if row % 2 != 0 { hex_w / 2.0 } else { 0.0 };

        for col in -3..cols {
            // Per-hexagon jitter and size variation
            let cell_r = base_cell_r * rng.gen_range(0.88..1.12);
            let jx = rng.gen_range(-base_cell_r * 0.08..base_cell_r * 0.08);
            let jy = rng.gen_range(-base_cell_r * 0.08..base_cell_r * 0.08);
            let raw_cx = col as f32 * hex_w + x_stagger + jx;
            let raw_cy = y_off + jy;
            let dx = raw_cx - center_x;
            let dy = raw_cy - center_y;
            let rot_cx = center_x + dx * grid_rotation.cos() - dy * grid_rotation.sin();
            let rot_cy = center_y + dx * grid_rotation.sin() + dy * grid_rotation.cos();

            // Randomly skip ~5% of hexagons for organic gaps
            if rng.gen_range(0.0..1.0_f32) < 0.05 {
                continue;
            }
            draw_hexagon(canvas, rot_cx, rot_cy, cell_r, color);
        }
    }
}

// ── Plume — flowing feather curves ──────────────────────────────────────────

fn draw_plume(canvas: &mut Canvas, w: f32, h: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Scatter plumes across the surface using a loose jittered grid.
    // Cell size varies randomly so the pattern doesn't look regular.
    let base_cell = 180.0_f32;
    let cols = (w / base_cell).ceil() as usize + 1;
    let rows = (h / base_cell).ceil() as usize + 1;

    for row in 0..rows {
        for col in 0..cols {
            // Heavily jittered position — can drift well outside its cell
            let cell_w = base_cell * rng.gen_range(0.7..1.4);
            let cell_h = base_cell * rng.gen_range(0.7..1.4);
            let ox = col as f32 * base_cell + rng.gen_range(-cell_w * 0.3..cell_w * 1.1);
            let oy = row as f32 * base_cell + rng.gen_range(-cell_h * 0.3..cell_h * 1.1);

            let angle: f32 = rng.gen_range(0.0..2.0 * PI);
            let spine_len = rng.gen_range(100.0..250.0_f32);
            // S-curve: two curvature phases so the spine bends back
            let curv1: f32 = rng.gen_range(-0.8..0.8);
            let curv2: f32 = rng.gen_range(-0.6..0.6);

            let steps = 800_u32;
            let barb_count = rng.gen_range(20..40_u32);
            let barb_len = rng.gen_range(25.0..60.0_f32);

            // Draw spine with two-phase curvature for organic S-shape
            let mut spine_points = Vec::with_capacity(steps as usize);
            for i in 0..steps {
                let t = i as f32 / steps as f32;
                let curv = if t < 0.5 {
                    curv1 * (1.0 - t * 2.0) + curv2 * t * 2.0
                } else {
                    curv2
                };
                let a = angle + curv * t;
                let x = ox + spine_len * t * a.cos();
                let y = oy + spine_len * t * a.sin();
                canvas.blend_pixel(x as i32, y as i32, color);
                // Double-draw for slightly thicker spine
                canvas.blend_pixel(x as i32 + 1, y as i32, color);
                spine_points.push((x, y, a));
            }

            // Barbs with variable spacing (denser near base, sparser at tip)
            for b in 0..barb_count {
                // Non-uniform distribution along spine
                let t_pos = (b as f32 / barb_count as f32).powf(0.7);
                let idx = (t_pos * (spine_points.len() - 1) as f32) as usize;
                if idx >= spine_points.len() {
                    break;
                }
                let (sx, sy, sa) = spine_points[idx];
                let t_ratio = idx as f32 / spine_points.len() as f32;
                // Barbs taper toward tip, vary individually
                let len = barb_len * (1.0 - t_ratio * 0.6) * rng.gen_range(0.6..1.3);
                let barb_curve: f32 = rng.gen_range(-0.4..0.4);

                for side in &[-1.0_f32, 1.0] {
                    let barb_angle = sa + side * rng.gen_range(0.4..1.2);
                    let barb_steps = 120_u32;
                    for j in 0..barb_steps {
                        let bt = j as f32 / barb_steps as f32;
                        let ba = barb_angle + barb_curve * bt;
                        let bx = sx + len * bt * ba.cos();
                        let by = sy + len * bt * ba.sin();
                        canvas.blend_pixel(bx as i32, by as i32, color);
                    }
                }
            }
        }
    }
}

// ── Constellation — star nodes connected by fine web ────────────────────────

fn draw_constellation(canvas: &mut Canvas, w: f32, h: f32, color: Rgba<u8>, faint: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Jittered grid with variable cell size for organic distribution.
    let base_cell = 110.0_f32;
    let cols = (w / base_cell).ceil() as usize + 2;
    let rows = (h / base_cell).ceil() as usize + 2;

    let mut nodes: Vec<(f32, f32)> = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            // Heavy jitter — nodes can drift 50% outside their cell
            let x = col as f32 * base_cell + rng.gen_range(-base_cell * 0.4..base_cell * 1.2);
            let y = row as f32 * base_cell + rng.gen_range(-base_cell * 0.4..base_cell * 1.2);
            // Randomly skip ~15% of nodes for irregular gaps
            if rng.gen_range(0.0..1.0_f32) > 0.15 {
                nodes.push((x, y));
            }
        }
    }
    // Extra fully random nodes to break grid feel
    let extra = (nodes.len() as f32 * 0.15) as usize;
    for _ in 0..extra {
        nodes.push((
            rng.gen_range(-50.0..w + 50.0),
            rng.gen_range(-50.0..h + 50.0),
        ));
    }

    let max_dist = base_cell * 2.0;
    let max_dist_sq = max_dist * max_dist;

    for i in 0..nodes.len() {
        let (ax, ay) = nodes[i];

        let mut neighbours: Vec<(usize, f32)> = Vec::new();
        #[allow(clippy::needless_range_loop)]
        for j in 0..nodes.len() {
            if i == j {
                continue;
            }
            let (bx, by) = nodes[j];
            let dsq = (ax - bx) * (ax - bx) + (ay - by) * (ay - by);
            if dsq < max_dist_sq {
                neighbours.push((j, dsq));
            }
        }
        neighbours.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // Connect to 2-5 nearest neighbours (random per node)
        let link_count = rng.gen_range(2..6_usize).min(neighbours.len());
        for &(j, _) in neighbours.iter().take(link_count) {
            let (bx, by) = nodes[j];
            // Bigger curve offset for more organic connections
            let mid_x = (ax + bx) / 2.0 + rng.gen_range(-base_cell * 0.2..base_cell * 0.2);
            let mid_y = (ay + by) / 2.0 + rng.gen_range(-base_cell * 0.2..base_cell * 0.2);
            let line_steps = 200_u32;
            for s in 0..line_steps {
                let t = s as f32 / line_steps as f32;
                let it = 1.0 - t;
                let x = it * it * ax + 2.0 * it * t * mid_x + t * t * bx;
                let y = it * it * ay + 2.0 * it * t * mid_y + t * t * by;
                canvas.blend_pixel(x as i32, y as i32, faint);
                // Double-draw for visibility
                canvas.blend_pixel(x as i32, y as i32 + 1, faint);
            }
        }

        // Radial burst — variable ray count and length per node
        let num_rays = rng.gen_range(4..12_u32);
        let ray_len = rng.gen_range(15.0..40.0_f32);
        let base_angle: f32 = rng.gen_range(0.0..2.0 * PI);
        // Irregular spacing between rays
        for r in 0..num_rays {
            let angle =
                base_angle + r as f32 * 2.0 * PI / num_rays as f32 + rng.gen_range(-0.15..0.15);
            let this_len = ray_len * rng.gen_range(0.5..1.4);
            let ray_steps = 80_u32;
            for s in 0..ray_steps {
                let t = s as f32 / ray_steps as f32;
                let x = ax + this_len * t * angle.cos();
                let y = ay + this_len * t * angle.sin();
                canvas.blend_pixel(x as i32, y as i32, color);
            }
        }

        // Node center — randomly circle or filled dot
        let node_r = rng.gen_range(2..7_i32);
        if rng.gen_bool(0.4) {
            canvas.fill_circle(ax as i32, ay as i32, node_r, color);
        } else {
            canvas.draw_circle(ax as i32, ay as i32, node_r, color);
        }
    }
}

// ── Ripple — overlapping elliptical wave fronts ─────────────────────────────

fn draw_ripple(canvas: &mut Canvas, w: f32, h: f32, color: Rgba<u8>) {
    let mut rng = rand::thread_rng();
    // Heavily jittered grid + random extras for organic placement.
    let base_cell = 250.0_f32;
    let cols = (w / base_cell).ceil() as usize + 2;
    let rows = (h / base_cell).ceil() as usize + 2;

    let mut origins: Vec<(f32, f32)> = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            // Large jitter so origins don't align
            let x = col as f32 * base_cell + rng.gen_range(-base_cell * 0.4..base_cell * 1.2);
            let y = row as f32 * base_cell + rng.gen_range(-base_cell * 0.4..base_cell * 1.2);
            origins.push((x, y));
        }
    }
    // Scatter extra random origins
    let extra = rng.gen_range(4..10_usize);
    for _ in 0..extra {
        origins.push((
            rng.gen_range(-100.0..w + 100.0),
            rng.gen_range(-100.0..h + 100.0),
        ));
    }

    for &(ox, oy) in &origins {
        // Each origin has its own max radius, spacing, eccentricity
        let max_r = rng.gen_range(base_cell * 0.8..base_cell * 1.8);
        let ring_spacing = rng.gen_range(12.0..24.0_f32);
        let stretch_x: f32 = rng.gen_range(0.65..1.35);
        let stretch_y: f32 = rng.gen_range(0.65..1.35);
        let rot: f32 = rng.gen_range(0.0..PI);
        let decay = rng.gen_range(0.002..0.005);
        let wobble_freq = rng.gen_range(2.0..9.0);

        let mut r = ring_spacing * rng.gen_range(0.5..1.5); // Random starting radius
        while r < max_r {
            let alpha_factor = (-decay * r).exp();
            if alpha_factor < 0.06 {
                break;
            }
            let ring_color = Rgba([
                color[0],
                color[1],
                color[2],
                (color[3] as f32 * alpha_factor).clamp(0.0, 255.0) as u8,
            ]);

            let circumference = 2.0 * PI * r;
            let steps = (circumference * 0.9).max(150.0) as u32;
            let wobble_amp = r * rng.gen_range(0.015..0.04);

            for i in 0..steps {
                let theta = i as f32 * 2.0 * PI / steps as f32;
                let rr = r + wobble_amp * (wobble_freq * theta).sin();
                let ex = rr * stretch_x * theta.cos();
                let ey = rr * stretch_y * theta.sin();
                let x = ox + ex * rot.cos() - ey * rot.sin();
                let y = oy + ex * rot.sin() + ey * rot.cos();
                canvas.blend_pixel(x as i32, y as i32, ring_color);
            }

            // Slightly irregular spacing between rings
            r += ring_spacing * rng.gen_range(0.8..1.2);
        }
    }
}

fn draw_hexagon(canvas: &mut Canvas, cx: f32, cy: f32, r: f32, color: Rgba<u8>) {
    let mut pts = [(0i32, 0i32); 6];
    #[allow(clippy::needless_range_loop)]
    for i in 0..6 {
        let angle = PI / 6.0 + i as f32 * PI / 3.0;
        pts[i] = ((cx + r * angle.cos()) as i32, (cy + r * angle.sin()) as i32);
    }
    for i in 0..6 {
        let j = (i + 1) % 6;
        canvas.draw_line(pts[i].0, pts[i].1, pts[j].0, pts[j].1, color);
    }
}
