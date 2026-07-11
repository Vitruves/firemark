use image::RgbaImage;

/// Block-based saliency map of an image.
///
/// Scores each block by its gradient energy (sum of absolute horizontal and
/// vertical pixel differences). High-scoring blocks contain text, edges, or
/// figure detail; low-scoring blocks are flat background. Watermark placement
/// biased toward salient blocks forces AI inpainting to reconstruct real
/// content, making removal visibly lossy.
pub struct SaliencyMap {
    block: u32,
    cols: u32,
    rows: u32,
    scores: Vec<f32>,
}

impl SaliencyMap {
    pub fn from_image(img: &RgbaImage) -> Self {
        let (w, h) = (img.width(), img.height());
        let block = (w.min(h) / 48).clamp(8, 64);
        let cols = w.div_ceil(block).max(1);
        let rows = h.div_ceil(block).max(1);
        let mut scores = vec![0.0f32; (cols * rows) as usize];

        for y in 0..h {
            let row = (y / block).min(rows - 1);
            for x in 0..w {
                let col = (x / block).min(cols - 1);
                let px = img.get_pixel(x, y);
                let mut grad = 0u32;
                if x > 0 {
                    let left = img.get_pixel(x - 1, y);
                    grad += abs_diff_rgb(px, left);
                }
                if y > 0 {
                    let up = img.get_pixel(x, y - 1);
                    grad += abs_diff_rgb(px, up);
                }
                scores[(row * cols + col) as usize] += grad as f32;
            }
        }

        // Normalize against the 95th-percentile block so a few extreme blocks
        // don't flatten the rest of the map.
        let mut sorted: Vec<f32> = scores.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95 = sorted[(sorted.len() * 95 / 100).min(sorted.len() - 1)].max(1.0);
        for s in &mut scores {
            *s = (*s / p95).min(1.0);
        }

        Self {
            block,
            cols,
            rows,
            scores,
        }
    }

    /// Saliency in [0, 1] at pixel coordinates; out-of-bounds returns 0.
    pub fn score_at(&self, x: i32, y: i32) -> f32 {
        if x < 0 || y < 0 {
            return 0.0;
        }
        let col = (x as u32 / self.block).min(self.cols - 1);
        let row = (y as u32 / self.block).min(self.rows - 1);
        self.scores[(row * self.cols + col) as usize]
    }

    /// Mean saliency over a pixel-coordinate rectangle, sampled per block.
    pub fn region_score(&self, x: i32, y: i32, w: u32, h: u32) -> f32 {
        let step = self.block as i32;
        let mut sum = 0.0;
        let mut n = 0u32;
        let mut sy = y;
        while sy < y + h as i32 {
            let mut sx = x;
            while sx < x + w as i32 {
                sum += self.score_at(sx, sy);
                n += 1;
                sx += step;
            }
            sy += step;
        }
        if n == 0 {
            0.0
        } else {
            sum / n as f32
        }
    }
}

fn abs_diff_rgb(a: &image::Rgba<u8>, b: &image::Rgba<u8>) -> u32 {
    (a[0] as i32 - b[0] as i32).unsigned_abs()
        + (a[1] as i32 - b[1] as i32).unsigned_abs()
        + (a[2] as i32 - b[2] as i32).unsigned_abs()
}

/// Per-row horizontal gradient energy — a strong proxy for text lines.
pub fn row_gradient_density(img: &RgbaImage) -> Vec<f64> {
    let (w, h) = (img.width(), img.height());
    let mut density = vec![0.0f64; h as usize];

    for y in 0..h {
        let mut row_sum = 0u64;
        for x in 1..w {
            let left = img.get_pixel(x - 1, y);
            let right = img.get_pixel(x, y);
            let diff = (left[0] as i32 - right[0] as i32).unsigned_abs()
                + (left[1] as i32 - right[1] as i32).unsigned_abs()
                + (left[2] as i32 - right[2] as i32).unsigned_abs();
            row_sum += diff as u64;
        }
        density[y as usize] = row_sum as f64;
    }

    density
}

/// Smooth a per-row density profile with a running box filter of `kernel` rows.
pub fn smooth_density(density: &[f64], kernel: usize) -> Vec<f64> {
    let n = density.len();
    let kernel = kernel.max(1);
    let mut smoothed = vec![0.0f64; n];
    let mut running_sum: f64 = density[..kernel.min(n)].iter().sum();
    for i in 0..n {
        let add = if i + kernel < n {
            density[i + kernel]
        } else {
            0.0
        };
        let sub = if i >= kernel {
            density[i - kernel]
        } else {
            0.0
        };
        running_sum += add - sub;
        smoothed[i] = running_sum;
    }
    smoothed
}
