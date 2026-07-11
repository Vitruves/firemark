use image::Rgba;

use crate::error::{FiremarkError, Result};

/// Parse a CSS color string (hex, named, rgb(), etc.) into `[R, G, B, A]`.
pub fn parse_color(s: &str) -> Result<[u8; 4]> {
    let c = csscolorparser::parse(s)
        .map_err(|e| FiremarkError::InvalidArgument(format!("Invalid color '{s}': {e}")))?;
    let [r, g, b, a] = c.to_rgba8();
    Ok([r, g, b, a])
}

/// Apply an opacity multiplier (0.0 -- 1.0) to the alpha channel.
pub fn with_opacity(color: [u8; 4], opacity: f32) -> [u8; 4] {
    let a = (color[3] as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
    [color[0], color[1], color[2], a]
}

/// Invert the RGB channels, keeping alpha unchanged.
pub fn invert_color(color: [u8; 4]) -> [u8; 4] {
    [255 - color[0], 255 - color[1], 255 - color[2], color[3]]
}

/// Convert to perceptual grayscale (ITU-R BT.601), keeping alpha unchanged.
pub fn to_grayscale(color: [u8; 4]) -> [u8; 4] {
    let gray = (0.299 * color[0] as f32 + 0.587 * color[1] as f32 + 0.114 * color[2] as f32)
        .round() as u8;
    [gray, gray, gray, color[3]]
}

/// Convert a `[u8; 4]` color array to an `image::Rgba<u8>`.
pub fn to_rgba(color: [u8; 4]) -> Rgba<u8> {
    Rgba(color)
}

/// Perceptual luminance (ITU-R BT.601) of an RGBA pixel, ignoring alpha.
pub fn luminance(px: &Rgba<u8>) -> f32 {
    0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32
}

/// Adapt an ink color to the local background so the mark has no constant
/// color signature to segment on: darken it over light backgrounds, lighten
/// it over dark ones, leave midtones alone. Alpha is preserved.
pub fn adaptive_ink(color: [u8; 4], bg_luminance: f32) -> [u8; 4] {
    if bg_luminance > 150.0 {
        let t = ((bg_luminance - 150.0) / 105.0).clamp(0.0, 1.0) * 0.55;
        lerp(color, [0, 0, 0, color[3]], t)
    } else if bg_luminance < 90.0 {
        let t = ((90.0 - bg_luminance) / 90.0).clamp(0.0, 1.0) * 0.55;
        lerp(color, [255, 255, 255, color[3]], t)
    } else {
        color
    }
}

/// Linearly interpolate between two colors. `t` is clamped to `[0.0, 1.0]`.
pub fn lerp(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        lerp_u8(a[0], b[0], t),
        lerp_u8(a[1], b[1], t),
        lerp_u8(a[2], b[2], t),
        lerp_u8(a[3], b[3], t),
    ]
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}
