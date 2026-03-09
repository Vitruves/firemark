use std::path::PathBuf;

use crate::cli::args::{
    BackgroundPattern, BlendMode, BorderStyle, FiligraneStyle, FontStyle, FontWeight, Position,
};
use crate::cli::validators::PageRange;
use crate::watermark::WatermarkType;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WatermarkConfig {
    // ── Input / Output ──
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub suffix: Option<String>,
    pub recursive: bool,
    pub jobs: usize,
    pub overwrite: bool,
    pub dry_run: bool,

    // ── Watermark Type ──
    pub watermark_type: WatermarkType,

    // ── Content & Templates ──
    pub main_text: String,
    pub secondary_text: String,
    pub image_path: Option<PathBuf>,
    pub qr_data: Option<String>,
    pub template: Option<String>,

    // ── Typography ──
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: FontWeight,
    pub font_style: FontStyle,
    pub letter_spacing: f32,

    // ── Position & Layout ──
    pub position: Position,
    pub rotation: f32,
    pub margin: u32,
    pub scale: f32,
    pub tile_spacing: u32,
    pub tile_rows: Option<u32>,
    pub tile_cols: Option<u32>,
    pub offset: (i32, i32),

    // ── Style & Appearance ──
    pub color: [u8; 4],
    pub opacity: f32,
    pub background: BackgroundPattern,
    pub bg_color: [u8; 4],
    pub bg_opacity: f32,
    pub blend: BlendMode,
    pub border: bool,
    pub border_color: Option<[u8; 4]>,
    pub border_width: u32,
    pub border_style: BorderStyle,
    pub shadow: bool,
    pub shadow_color: [u8; 4],
    pub shadow_offset: (i32, i32),
    pub shadow_blur: u32,
    pub shadow_opacity: f32,
    pub invert: bool,
    pub grayscale: bool,
    pub filigrane: FiligraneStyle,
    pub anti_ai: bool,

    // ── PDF-specific ──
    #[serde(skip, default)]
    pub pages: PageRange,
    #[serde(skip, default)]
    pub skip_pages: Option<PageRange>,
    pub layer_name: String,
    pub flatten: bool,
    pub behind: bool,

    // ── Output Quality ──
    pub quality: u8,
    pub dpi: u32,
    pub strip_metadata: bool,
    pub png_compression: u8,
    pub color_profile: Option<PathBuf>,
}

impl Default for WatermarkConfig {
    fn default() -> Self {
        Self {
            input: PathBuf::new(),
            output: None,
            suffix: None,
            recursive: false,
            jobs: 1,
            overwrite: false,
            dry_run: false,

            watermark_type: WatermarkType::Diagonal,

            main_text: "firemark".to_string(),
            secondary_text: "{timestamp}".to_string(),
            image_path: None,
            qr_data: None,
            template: None,

            font: None,
            font_size: None,
            font_weight: FontWeight::Regular,
            font_style: FontStyle::Normal,
            letter_spacing: 0.0,

            position: Position::Center,
            rotation: -45.0,
            margin: 20,
            scale: 0.4,
            tile_spacing: 80,
            tile_rows: None,
            tile_cols: None,
            offset: (0, 0),

            color: [0x80, 0x80, 0x80, 0xFF],
            opacity: 0.5,
            background: BackgroundPattern::None,
            bg_color: [0xCC, 0xCC, 0xCC, 0xFF],
            bg_opacity: 0.15,
            blend: BlendMode::Normal,
            border: false,
            border_color: None,
            border_width: 1,
            border_style: BorderStyle::Solid,
            shadow: false,
            shadow_color: [0x00, 0x00, 0x00, 0xFF],
            shadow_offset: (2, 2),
            shadow_blur: 4,
            shadow_opacity: 0.4,
            invert: false,
            grayscale: false,
            filigrane: FiligraneStyle::Guilloche,
            anti_ai: true,

            pages: PageRange::All,
            skip_pages: None,
            layer_name: "Watermark".to_string(),
            flatten: true,
            behind: false,

            quality: 90,
            dpi: 150,
            strip_metadata: false,
            png_compression: 6,
            color_profile: None,
        }
    }
}
