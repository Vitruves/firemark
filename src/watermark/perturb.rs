use image::RgbaImage;
use rand::Rng;

/// Universal post-render perturbation applied to the watermark overlay before
/// compositing onto the source image.
///
/// This makes every render non-deterministic and prevents pixel-perfect AI
/// removal by introducing subtle, invisible randomness:
///   1. Alpha jitter on every non-transparent pixel
///   2. Sub-pixel color noise on every non-transparent pixel
///   3. Edge micro-dots along the alpha boundary
///   4. Sparse ghost pixels across the full canvas
pub fn perturb(wm: &mut RgbaImage) {
    let mut rng = rand::thread_rng();
    let (w, h) = (wm.width(), wm.height());

    // Collect the dominant watermark color (average of visible pixels).
    let mut r_sum: u64 = 0;
    let mut g_sum: u64 = 0;
    let mut b_sum: u64 = 0;
    let mut count: u64 = 0;

    for px in wm.pixels() {
        if px[3] > 0 {
            r_sum += px[0] as u64;
            g_sum += px[1] as u64;
            b_sum += px[2] as u64;
            count += 1;
        }
    }

    let (dom_r, dom_g, dom_b) = if count > 0 {
        (
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
        )
    } else {
        return; // nothing to perturb
    };

    // --- Pass 1 & 2: Alpha jitter + sub-pixel color noise ---
    for px in wm.pixels_mut() {
        if px[3] == 0 {
            continue;
        }

        // Alpha jitter: +-10, clamped to [1, 255]
        let a = px[3] as i16 + rng.gen_range(-10i16..=10);
        px[3] = a.clamp(1, 255) as u8;

        // Sub-pixel color noise: +-4 per channel
        let r = px[0] as i16 + rng.gen_range(-4i16..=4);
        let g = px[1] as i16 + rng.gen_range(-4i16..=4);
        let b = px[2] as i16 + rng.gen_range(-4i16..=4);
        px[0] = r.clamp(0, 255) as u8;
        px[1] = g.clamp(0, 255) as u8;
        px[2] = b.clamp(0, 255) as u8;
    }

    // --- Pass 3: Edge micro-dots ---
    // We need a snapshot of alpha to detect boundaries.
    let alpha_snap: Vec<u8> = wm.pixels().map(|px| px[3]).collect();
    let get_alpha = |x: i32, y: i32| -> u8 {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            0
        } else {
            alpha_snap[y as usize * w as usize + x as usize]
        }
    };

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let a = get_alpha(x, y);
            if a > 0 {
                // Check if this pixel is on an alpha boundary (adjacent to alpha=0)
                let is_edge = get_alpha(x - 1, y) == 0
                    || get_alpha(x + 1, y) == 0
                    || get_alpha(x, y - 1) == 0
                    || get_alpha(x, y + 1) == 0;

                if is_edge && rng.gen::<f32>() < 0.03 {
                    // Average color of neighboring watermark pixels
                    let mut nr: u32 = 0;
                    let mut ng: u32 = 0;
                    let mut nb: u32 = 0;
                    let mut nc: u32 = 0;
                    for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && ny >= 0 && nx < w as i32 && ny < h as i32 {
                            let idx = (ny as u32 * w + nx as u32) as usize;
                            if alpha_snap[idx] > 0 {
                                let npx = wm.get_pixel(nx as u32, ny as u32);
                                nr += npx[0] as u32;
                                ng += npx[1] as u32;
                                nb += npx[2] as u32;
                                nc += 1;
                            }
                        }
                    }
                    if nc > 0 {
                        // Place a dot at the neighboring transparent pixel
                        for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
                            let nx = x + dx;
                            let ny = y + dy;
                            if nx >= 0
                                && ny >= 0
                                && nx < w as i32
                                && ny < h as i32
                                && get_alpha(nx, ny) == 0
                            {
                                let dot_alpha = rng.gen_range(20u8..=60);
                                wm.put_pixel(
                                    nx as u32,
                                    ny as u32,
                                    image::Rgba([
                                        (nr / nc) as u8,
                                        (ng / nc) as u8,
                                        (nb / nc) as u8,
                                        dot_alpha,
                                    ]),
                                );
                                break; // one dot per edge pixel
                            }
                        }
                    }
                }
            }
        }
    }

    // --- Pass 4: Sparse ghost pixels ---
    for y in 0..h {
        for x in 0..w {
            if rng.gen::<f32>() < 0.005 {
                let px = wm.get_pixel(x, y);
                if px[3] == 0 {
                    let ghost_alpha = rng.gen_range(3u8..=8);
                    wm.put_pixel(x, y, image::Rgba([dom_r, dom_g, dom_b, ghost_alpha]));
                }
            }
        }
    }
}
