use ab_glyph::FontArc;
use image::{Rgba, RgbaImage};
use imageproc::drawing;

/// Canvas wrapper around an `RgbaImage` providing convenient drawing operations.
pub struct Canvas {
    img: RgbaImage,
    width: u32,
    height: u32,
}

impl Canvas {
    /// Create a new transparent canvas with the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        let img = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
        Self { img, width, height }
    }

    /// Wrap an existing `RgbaImage` as a `Canvas`.
    pub fn from_image(img: RgbaImage) -> Self {
        let width = img.width();
        let height = img.height();
        Self { img, width, height }
    }

    /// Get a mutable reference to the pixel at `(x, y)`.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> &mut Rgba<u8> {
        self.img.get_pixel_mut(x, y)
    }

    /// Set the pixel at `(x, y)` if the coordinates are within bounds.
    pub fn set_pixel(&mut self, x: i32, y: i32, color: Rgba<u8>) {
        if x >= 0 && y >= 0 && (x as u32) < self.width && (y as u32) < self.height {
            *self.img.get_pixel_mut(x as u32, y as u32) = color;
        }
    }

    /// Alpha-blend a pixel onto the existing pixel at `(x, y)`.
    pub fn blend_pixel(&mut self, x: i32, y: i32, color: Rgba<u8>) {
        if x < 0 || y < 0 || (x as u32) >= self.width || (y as u32) >= self.height {
            return;
        }
        let dst = self.img.get_pixel_mut(x as u32, y as u32);
        let sa = color[3] as f32 / 255.0;
        let da = dst[3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a < 0.001 {
            return;
        }
        for i in 0..3 {
            let blended = (color[i] as f32 * sa + dst[i] as f32 * da * (1.0 - sa)) / out_a;
            dst[i] = blended.clamp(0.0, 255.0) as u8;
        }
        dst[3] = (out_a * 255.0).clamp(0.0, 255.0) as u8;
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn into_image(self) -> RgbaImage {
        self.img
    }

    pub fn image(&self) -> &RgbaImage {
        &self.img
    }

    pub fn image_mut(&mut self) -> &mut RgbaImage {
        &mut self.img
    }

    pub fn clear(&mut self, color: Rgba<u8>) {
        for pixel in self.img.pixels_mut() {
            *pixel = color;
        }
    }

    /// Draw text onto the canvas.
    pub fn draw_text(
        &mut self,
        font: &FontArc,
        text: &str,
        x: f32,
        y: f32,
        scale: f32,
        color: Rgba<u8>,
    ) {
        let px_scale = ab_glyph::PxScale::from(scale);
        drawing::draw_text_mut(
            &mut self.img,
            color,
            x as i32,
            y as i32,
            px_scale,
            font,
            text,
        );
    }

    /// Draw a line between two points.
    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Rgba<u8>) {
        drawing::draw_line_segment_mut(
            &mut self.img,
            (x1 as f32, y1 as f32),
            (x2 as f32, y2 as f32),
            color,
        );
    }

    /// Draw a thick line (multiple parallel lines).
    pub fn draw_thick_line(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        thickness: u32,
        color: Rgba<u8>,
    ) {
        if thickness <= 1 {
            self.draw_line(x1, y1, x2, y2, color);
            return;
        }
        let dx = (x2 - x1) as f32;
        let dy = (y2 - y1) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }
        // Normal perpendicular to the line direction
        let nx = -dy / len;
        let ny = dx / len;
        let half = thickness as f32 / 2.0;
        for i in 0..thickness {
            let offset = i as f32 - half + 0.5;
            let ox = (nx * offset) as i32;
            let oy = (ny * offset) as i32;
            self.draw_line(x1 + ox, y1 + oy, x2 + ox, y2 + oy, color);
        }
    }

    /// Draw a dashed line.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_dashed_line(
        &mut self,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        dash_len: u32,
        gap_len: u32,
        color: Rgba<u8>,
    ) {
        let dx = (x2 - x1) as f32;
        let dy = (y2 - y1) as f32;
        let total_len = (dx * dx + dy * dy).sqrt();
        if total_len < 1.0 {
            return;
        }
        let ux = dx / total_len;
        let uy = dy / total_len;
        let cycle = (dash_len + gap_len) as f32;
        let mut t = 0.0f32;
        while t < total_len {
            let end = (t + dash_len as f32).min(total_len);
            let sx = x1 as f32 + ux * t;
            let sy = y1 as f32 + uy * t;
            let ex = x1 as f32 + ux * end;
            let ey = y1 as f32 + uy * end;
            self.draw_line(sx as i32, sy as i32, ex as i32, ey as i32, color);
            t += cycle;
        }
    }

    /// Draw a hollow rectangle.
    pub fn draw_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Rgba<u8>) {
        if w == 0 || h == 0 {
            return;
        }
        let rect = imageproc::rect::Rect::at(x, y).of_size(w, h);
        drawing::draw_hollow_rect_mut(&mut self.img, rect, color);
    }

    /// Draw a filled rectangle.
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, color: Rgba<u8>) {
        if w == 0 || h == 0 {
            return;
        }
        let rect = imageproc::rect::Rect::at(x, y).of_size(w, h);
        drawing::draw_filled_rect_mut(&mut self.img, rect, color);
    }

    /// Draw a hollow circle.
    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
        if radius <= 0 {
            return;
        }
        drawing::draw_hollow_circle_mut(&mut self.img, (cx, cy), radius, color);
    }

    /// Draw a filled circle.
    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
        if radius <= 0 {
            return;
        }
        drawing::draw_filled_circle_mut(&mut self.img, (cx, cy), radius, color);
    }

    /// Draw a thick circle outline (multiple concentric circles).
    pub fn draw_thick_circle(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        thickness: u32,
        color: Rgba<u8>,
    ) {
        let half = thickness as i32 / 2;
        for i in 0..thickness as i32 {
            let r = radius - half + i;
            if r > 0 {
                self.draw_circle(cx, cy, r, color);
            }
        }
    }

    /// Draw a polygon outline from a list of points.
    pub fn draw_polygon(&mut self, points: &[(i32, i32)], color: Rgba<u8>) {
        if points.len() < 2 {
            return;
        }
        for i in 0..points.len() {
            let (x1, y1) = points[i];
            let (x2, y2) = points[(i + 1) % points.len()];
            self.draw_line(x1, y1, x2, y2, color);
        }
    }

    /// Draw a filled polygon.
    pub fn fill_polygon(&mut self, points: &[(i32, i32)], color: Rgba<u8>) {
        if points.len() < 3 {
            return;
        }
        // Find bounding box
        let min_y = points.iter().map(|p| p.1).min().unwrap();
        let max_y = points.iter().map(|p| p.1).max().unwrap();
        let min_x = points.iter().map(|p| p.0).min().unwrap();
        let max_x = points.iter().map(|p| p.0).max().unwrap();

        // Scanline fill
        for y in min_y..=max_y {
            let mut intersections = Vec::new();
            let n = points.len();
            for i in 0..n {
                let (x1, y1) = (points[i].0 as f32, points[i].1 as f32);
                let (x2, y2) = (points[(i + 1) % n].0 as f32, points[(i + 1) % n].1 as f32);
                let yf = y as f32;
                if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                    let x_intersect = x1 + (yf - y1) / (y2 - y1) * (x2 - x1);
                    intersections.push(x_intersect as i32);
                }
            }
            intersections.sort();
            for pair in intersections.chunks(2) {
                if pair.len() == 2 {
                    let start = pair[0].max(min_x);
                    let end = pair[1].min(max_x);
                    for x in start..=end {
                        self.set_pixel(x, y, color);
                    }
                }
            }
        }
    }

    /// Draw a regular star with given number of points.
    pub fn draw_star(
        &mut self,
        cx: i32,
        cy: i32,
        outer_radius: i32,
        inner_radius: i32,
        num_points: u32,
        color: Rgba<u8>,
    ) {
        let points = star_points(cx, cy, outer_radius, inner_radius, num_points);
        self.draw_polygon(&points, color);
    }

    /// Draw a filled star.
    pub fn fill_star(
        &mut self,
        cx: i32,
        cy: i32,
        outer_radius: i32,
        inner_radius: i32,
        num_points: u32,
        color: Rgba<u8>,
    ) {
        let points = star_points(cx, cy, outer_radius, inner_radius, num_points);
        self.fill_polygon(&points, color);
    }

    /// Draw text along a circular arc. Characters are placed individually, rotated tangent to the circle.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_on_arc(
        &mut self,
        font: &FontArc,
        text: &str,
        cx: f32,
        cy: f32,
        radius: f32,
        start_angle: f32,
        scale: f32,
        color: Rgba<u8>,
    ) {
        use ab_glyph::{Font, PxScale, ScaleFont};
        let px_scale = PxScale::from(scale);
        let scaled_font = font.as_scaled(px_scale);

        // Calculate total arc length needed for text
        let chars: Vec<char> = text.chars().collect();
        let total_width: f32 = chars
            .iter()
            .map(|&c| scaled_font.h_advance(font.glyph_id(c)))
            .sum();

        // Angular span = arc_length / radius
        let total_angle = total_width / radius;
        let mut angle = start_angle - total_angle / 2.0;

        for &ch in &chars {
            let advance = scaled_font.h_advance(font.glyph_id(ch));
            let char_angle = advance / (2.0 * radius);
            angle += char_angle;

            // Position on circle
            let px = cx + radius * angle.cos();
            let py = cy + radius * angle.sin();

            // Draw the character (simplified — no rotation per char, just placement)
            let s = ch.to_string();
            self.draw_text(font, &s, px - advance / 2.0, py - scale / 2.0, scale, color);

            angle += char_angle;
        }
    }

    /// Blit another canvas onto this one at (x, y), alpha blending.
    pub fn blit(&mut self, other: &Canvas, x: i32, y: i32) {
        let src = other.image();
        for sy in 0..other.height() {
            for sx in 0..other.width() {
                let px = *src.get_pixel(sx, sy);
                if px[3] > 0 {
                    self.blend_pixel(x + sx as i32, y + sy as i32, px);
                }
            }
        }
    }

    /// Blit with simple copy (no blending) for fully opaque overlay.
    pub fn blit_opaque(&mut self, other: &Canvas, x: i32, y: i32) {
        let src = other.image();
        for sy in 0..other.height() {
            for sx in 0..other.width() {
                let px = *src.get_pixel(sx, sy);
                if px[3] > 0 {
                    self.set_pixel(x + sx as i32, y + sy as i32, px);
                }
            }
        }
    }
}

/// Generate vertices of a star shape.
fn star_points(
    cx: i32,
    cy: i32,
    outer_radius: i32,
    inner_radius: i32,
    num_points: u32,
) -> Vec<(i32, i32)> {
    let mut points = Vec::with_capacity(num_points as usize * 2);
    let total = num_points * 2;
    let start_angle = -std::f32::consts::FRAC_PI_2; // Start at top
    for i in 0..total {
        let angle = start_angle + std::f32::consts::PI * 2.0 * i as f32 / total as f32;
        let r = if i % 2 == 0 {
            outer_radius as f32
        } else {
            inner_radius as f32
        };
        let x = cx as f32 + r * angle.cos();
        let y = cy as f32 + r * angle.sin();
        points.push((x as i32, y as i32));
    }
    points
}
