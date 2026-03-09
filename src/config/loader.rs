use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::cli::args::{
    BackgroundPattern, BlendMode, BorderStyle, FiligraneStyle, FontStyle, FontWeight, Position,
};
use crate::watermark::WatermarkType;

/// Represents the full TOML configuration file.
///
/// Top-level keys map to global defaults. The `[preset.<name>]` table
/// holds named presets that can override any subset of fields.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct TomlConfig {
    // ── Input / Output ──
    pub output: Option<PathBuf>,
    pub suffix: Option<String>,
    pub recursive: Option<bool>,
    pub jobs: Option<usize>,
    pub overwrite: Option<bool>,

    // ── Watermark Type ──
    pub watermark_type: Option<WatermarkType>,

    // ── Content & Templates ──
    pub main_text: Option<String>,
    pub secondary_text: Option<String>,
    pub image_path: Option<PathBuf>,
    pub qr_data: Option<String>,
    pub template: Option<String>,

    // ── Typography ──
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub letter_spacing: Option<f32>,

    // ── Position & Layout ──
    pub position: Option<Position>,
    pub rotation: Option<f32>,
    pub margin: Option<u32>,
    pub scale: Option<f32>,
    pub tile_spacing: Option<u32>,
    pub tile_rows: Option<u32>,
    pub tile_cols: Option<u32>,
    pub offset: Option<[i32; 2]>,

    // ── Style & Appearance ──
    pub color: Option<String>,
    pub opacity: Option<f32>,
    pub background: Option<BackgroundPattern>,
    pub bg_color: Option<String>,
    pub bg_opacity: Option<f32>,
    pub blend: Option<BlendMode>,
    pub border: Option<bool>,
    pub border_color: Option<String>,
    pub border_width: Option<u32>,
    pub border_style: Option<BorderStyle>,
    pub shadow: Option<bool>,
    pub shadow_color: Option<String>,
    pub shadow_offset: Option<[i32; 2]>,
    pub shadow_blur: Option<u32>,
    pub shadow_opacity: Option<f32>,
    pub invert: Option<bool>,
    pub grayscale: Option<bool>,
    pub filigrane: Option<FiligraneStyle>,
    pub anti_ai: Option<bool>,

    // ── PDF-specific ──
    pub pages: Option<String>,
    pub skip_pages: Option<String>,
    pub layer_name: Option<String>,
    pub flatten: Option<bool>,
    pub behind: Option<bool>,

    // ── Output Quality ──
    pub quality: Option<u8>,
    pub dpi: Option<u32>,
    pub strip_metadata: Option<bool>,
    pub png_compression: Option<u8>,
    pub color_profile: Option<PathBuf>,

    // ── Presets ──
    #[serde(default)]
    pub preset: HashMap<String, PresetConfig>,
}

/// A named preset. Every field is optional so users can override only what they need.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PresetConfig {
    // ── Input / Output ──
    pub output: Option<PathBuf>,
    pub suffix: Option<String>,
    pub recursive: Option<bool>,
    pub jobs: Option<usize>,
    pub overwrite: Option<bool>,

    // ── Watermark Type ──
    pub watermark_type: Option<WatermarkType>,

    // ── Content & Templates ──
    pub main_text: Option<String>,
    pub secondary_text: Option<String>,
    pub image_path: Option<PathBuf>,
    pub qr_data: Option<String>,
    pub template: Option<String>,

    // ── Typography ──
    pub font: Option<String>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub font_style: Option<FontStyle>,
    pub letter_spacing: Option<f32>,

    // ── Position & Layout ──
    pub position: Option<Position>,
    pub rotation: Option<f32>,
    pub margin: Option<u32>,
    pub scale: Option<f32>,
    pub tile_spacing: Option<u32>,
    pub tile_rows: Option<u32>,
    pub tile_cols: Option<u32>,
    pub offset: Option<[i32; 2]>,

    // ── Style & Appearance ──
    pub color: Option<String>,
    pub opacity: Option<f32>,
    pub background: Option<BackgroundPattern>,
    pub bg_color: Option<String>,
    pub bg_opacity: Option<f32>,
    pub blend: Option<BlendMode>,
    pub border: Option<bool>,
    pub border_color: Option<String>,
    pub border_width: Option<u32>,
    pub border_style: Option<BorderStyle>,
    pub shadow: Option<bool>,
    pub shadow_color: Option<String>,
    pub shadow_offset: Option<[i32; 2]>,
    pub shadow_blur: Option<u32>,
    pub shadow_opacity: Option<f32>,
    pub invert: Option<bool>,
    pub grayscale: Option<bool>,
    pub filigrane: Option<FiligraneStyle>,
    pub anti_ai: Option<bool>,

    // ── PDF-specific ──
    pub pages: Option<String>,
    pub skip_pages: Option<String>,
    pub layer_name: Option<String>,
    pub flatten: Option<bool>,
    pub behind: Option<bool>,

    // ── Output Quality ──
    pub quality: Option<u8>,
    pub dpi: Option<u32>,
    pub strip_metadata: Option<bool>,
    pub png_compression: Option<u8>,
    pub color_profile: Option<PathBuf>,
}

/// Load and parse a TOML configuration file.
pub fn load_config(path: &Path) -> Result<TomlConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: TomlConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// Print all available preset names from the config file.
pub fn list_presets(config_path: &Option<PathBuf>) -> Result<()> {
    let path = match config_path {
        Some(p) => p.clone(),
        None => default_config_path()?,
    };

    if !path.exists() {
        println!("No config file found at: {}", path.display());
        println!("Create one to define presets.");
        return Ok(());
    }

    let config = load_config(&path)?;

    if config.preset.is_empty() {
        println!("No presets defined in {}", path.display());
    } else {
        println!("Available presets ({}):", path.display());
        let mut names: Vec<&String> = config.preset.keys().collect();
        names.sort();
        for name in names {
            println!("  - {name}");
        }
    }

    Ok(())
}

/// Return the default config file path: `~/.config/firemark/config.toml`
pub fn default_config_path() -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Could not determine home directory")?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("firemark")
        .join("config.toml"))
}
