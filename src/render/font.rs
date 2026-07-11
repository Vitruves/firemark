use ab_glyph::{Font, FontArc, PxScale, ScaleFont};

use crate::cli::args::FontWeight;
use crate::error::{FiremarkError, Result};

// Embedded Liberation fonts from the assets directory.
static LIBERATION_SANS_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/LiberationSans-Regular.ttf");
static LIBERATION_SANS_BOLD: &[u8] = include_bytes!("../../assets/fonts/LiberationSans-Bold.ttf");
static LIBERATION_MONO_REGULAR: &[u8] =
    include_bytes!("../../assets/fonts/LiberationMono-Regular.ttf");

/// Load a font by name or file path, selecting the appropriate weight.
///
/// - `None`, `"sans"`, or `"default"` returns the embedded Liberation Sans
///   (Regular or Bold depending on `weight`).
/// - `"mono"` returns the embedded Liberation Mono Regular.
/// - A path containing `'.'` or `'/'` is loaded from the filesystem.
/// - Anything else falls back to the embedded sans font.
pub fn load_font(name: Option<&str>, weight: FontWeight) -> Result<FontArc> {
    match name {
        None | Some("sans") | Some("default") => load_embedded_sans(weight),
        Some("mono") => FontArc::try_from_slice(LIBERATION_MONO_REGULAR)
            .map_err(|e| FiremarkError::Font(format!("Failed to load embedded mono font: {e}"))),
        Some(s) if s.contains('.') || s.contains('/') => load_from_file(s),
        Some(_) => load_embedded_sans(weight),
    }
}

/// Measure the rendered width and height of `text` at the given scale.
///
/// Returns `(width, height)` in pixels.
pub fn measure_text(font: &FontArc, text: &str, scale: f32) -> (f32, f32) {
    let px_scale = PxScale::from(scale);
    let scaled = font.as_scaled(px_scale);

    let mut width: f32 = 0.0;
    let mut prev: Option<ab_glyph::GlyphId> = None;

    for ch in text.chars() {
        let glyph_id = font.glyph_id(ch);
        if let Some(prev_id) = prev {
            width += scaled.kern(prev_id, glyph_id);
        }
        width += scaled.h_advance(glyph_id);
        prev = Some(glyph_id);
    }

    let height = scaled.height();
    (width, height)
}

/// Compute a font scale so that `text` fills approximately `target_ratio` of `canvas_width`.
pub fn auto_scale(text: &str, canvas_width: u32, target_ratio: f32, font: &FontArc) -> f32 {
    let target_width = canvas_width as f32 * target_ratio;

    // Start with a reasonable initial scale and iterate to converge.
    let mut scale: f32 = 40.0;
    for _ in 0..10 {
        let (w, _) = measure_text(font, text, scale);
        if w <= 0.0 {
            break;
        }
        scale *= target_width / w;
    }
    scale.max(1.0)
}

// ── Internal helpers ──

fn load_embedded_sans(weight: FontWeight) -> Result<FontArc> {
    let data = match weight {
        FontWeight::Bold | FontWeight::Black => LIBERATION_SANS_BOLD,
        _ => LIBERATION_SANS_REGULAR,
    };
    FontArc::try_from_slice(data)
        .map_err(|e| FiremarkError::Font(format!("Failed to load embedded sans font: {e}")))
}

fn load_from_file(path: &str) -> Result<FontArc> {
    let bytes = std::fs::read(path)
        .map_err(|e| FiremarkError::Font(format!("Failed to read font file '{path}': {e}")))?;
    FontArc::try_from_vec(bytes)
        .map_err(|e| FiremarkError::Font(format!("Failed to parse font file '{path}': {e}")))
}
