use anyhow::{Context, Result};

use crate::cli::args::CliArgs;
use crate::cli::validators::{parse_color, parse_offset, parse_page_range};
use crate::config::loader::{default_config_path, load_config, PresetConfig, TomlConfig};
use crate::config::types::WatermarkConfig;

/// Build a fully-resolved `WatermarkConfig` by layering:
///   defaults -> TOML globals -> preset -> CLI args
///
/// If `--save-preset` is provided, the current CLI flags are persisted
/// into the config file before returning.
pub fn resolve_config(args: &CliArgs) -> Result<WatermarkConfig> {
    let mut config = WatermarkConfig::default();

    // ── 1. Load TOML config (if any) ──
    let toml_config = load_toml_config(args)?;

    // ── 2. Merge global TOML values onto defaults ──
    if let Some(ref toml) = toml_config {
        merge_toml_globals(&mut config, toml)?;
    }

    // ── 3. Merge preset (if requested) ──
    if let Some(ref preset_name) = args.preset {
        let toml = toml_config
            .as_ref()
            .context("Cannot use --preset without a config file")?;
        let preset = toml
            .preset
            .get(preset_name)
            .with_context(|| format!("Preset '{preset_name}' not found in config file"))?;
        merge_preset(&mut config, preset)?;
    }

    // ── 4. Merge CLI args (highest priority) ──
    merge_cli_args(&mut config, args)?;

    // ── 5. Handle --save-preset ──
    if let Some(ref preset_name) = args.save_preset {
        save_preset(args, preset_name)?;
    }

    Ok(config)
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

fn load_toml_config(args: &CliArgs) -> Result<Option<TomlConfig>> {
    if let Some(ref path) = args.config {
        return Ok(Some(load_config(path)?));
    }

    // Try the default path silently
    if let Ok(path) = default_config_path() {
        if path.exists() {
            return Ok(Some(load_config(&path)?));
        }
    }

    Ok(None)
}

fn merge_toml_globals(config: &mut WatermarkConfig, toml: &TomlConfig) -> Result<()> {
    if let Some(ref v) = toml.output {
        config.output = Some(v.clone());
    }
    if let Some(ref v) = toml.suffix {
        config.suffix = Some(v.clone());
    }
    if let Some(v) = toml.recursive {
        config.recursive = v;
    }
    if let Some(v) = toml.jobs {
        config.jobs = v;
    }
    if let Some(v) = toml.overwrite {
        config.overwrite = v;
    }
    if let Some(v) = toml.watermark_type {
        config.watermark_type = v;
    }
    if let Some(ref v) = toml.main_text {
        config.main_text = v.clone();
    }
    if let Some(ref v) = toml.secondary_text {
        config.secondary_text = v.clone();
    }
    if let Some(ref v) = toml.image_path {
        config.image_path = Some(v.clone());
    }
    if let Some(ref v) = toml.qr_data {
        config.qr_data = Some(v.clone());
    }
    if let Some(ref v) = toml.template {
        config.template = Some(v.clone());
    }
    if let Some(ref v) = toml.font {
        config.font = Some(v.clone());
    }
    if let Some(v) = toml.font_size {
        config.font_size = Some(v);
    }
    if let Some(v) = toml.font_weight {
        config.font_weight = v;
    }
    if let Some(v) = toml.font_style {
        config.font_style = v;
    }
    if let Some(v) = toml.letter_spacing {
        config.letter_spacing = v;
    }
    if let Some(v) = toml.position {
        config.position = v;
    }
    if let Some(v) = toml.rotation {
        config.rotation = v;
    }
    if let Some(v) = toml.margin {
        config.margin = v;
    }
    if let Some(v) = toml.scale {
        config.scale = v;
    }
    if let Some(v) = toml.tile_spacing {
        config.tile_spacing = v;
    }
    if let Some(v) = toml.tile_rows {
        config.tile_rows = Some(v);
    }
    if let Some(v) = toml.tile_cols {
        config.tile_cols = Some(v);
    }
    if let Some(ref v) = toml.offset {
        config.offset = (v[0], v[1]);
    }
    if let Some(ref v) = toml.color {
        config.color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = toml.opacity {
        config.opacity = v;
    }
    if let Some(v) = toml.background {
        config.background = v;
    }
    if let Some(ref v) = toml.bg_color {
        config.bg_color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = toml.bg_opacity {
        config.bg_opacity = v;
    }
    if let Some(v) = toml.blend {
        config.blend = v;
    }
    if let Some(v) = toml.border {
        config.border = v;
    }
    if let Some(ref v) = toml.border_color {
        config.border_color = Some(parse_color(v).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(v) = toml.border_width {
        config.border_width = v;
    }
    if let Some(v) = toml.border_style {
        config.border_style = v;
    }
    if let Some(v) = toml.shadow {
        config.shadow = v;
    }
    if let Some(ref v) = toml.shadow_color {
        config.shadow_color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = toml.shadow_offset {
        config.shadow_offset = (v[0], v[1]);
    }
    if let Some(v) = toml.shadow_blur {
        config.shadow_blur = v;
    }
    if let Some(v) = toml.shadow_opacity {
        config.shadow_opacity = v;
    }
    if let Some(v) = toml.invert {
        config.invert = v;
    }
    if let Some(v) = toml.grayscale {
        config.grayscale = v;
    }
    if let Some(v) = toml.filigrane {
        config.filigrane = v;
    }
    if let Some(v) = toml.anti_ai {
        config.anti_ai = v;
    }
    if let Some(ref v) = toml.pages {
        config.pages = parse_page_range(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = toml.skip_pages {
        config.skip_pages = Some(parse_page_range(v).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(ref v) = toml.layer_name {
        config.layer_name = v.clone();
    }
    if let Some(v) = toml.flatten {
        config.flatten = v;
    }
    if let Some(v) = toml.behind {
        config.behind = v;
    }
    if let Some(v) = toml.quality {
        config.quality = v;
    }
    if let Some(v) = toml.dpi {
        config.dpi = v;
    }
    if let Some(v) = toml.strip_metadata {
        config.strip_metadata = v;
    }
    if let Some(v) = toml.png_compression {
        config.png_compression = v;
    }
    if let Some(ref v) = toml.color_profile {
        config.color_profile = Some(v.clone());
    }

    Ok(())
}

fn merge_preset(config: &mut WatermarkConfig, preset: &PresetConfig) -> Result<()> {
    if let Some(ref v) = preset.output {
        config.output = Some(v.clone());
    }
    if let Some(ref v) = preset.suffix {
        config.suffix = Some(v.clone());
    }
    if let Some(v) = preset.recursive {
        config.recursive = v;
    }
    if let Some(v) = preset.jobs {
        config.jobs = v;
    }
    if let Some(v) = preset.overwrite {
        config.overwrite = v;
    }
    if let Some(v) = preset.watermark_type {
        config.watermark_type = v;
    }
    if let Some(ref v) = preset.main_text {
        config.main_text = v.clone();
    }
    if let Some(ref v) = preset.secondary_text {
        config.secondary_text = v.clone();
    }
    if let Some(ref v) = preset.image_path {
        config.image_path = Some(v.clone());
    }
    if let Some(ref v) = preset.qr_data {
        config.qr_data = Some(v.clone());
    }
    if let Some(ref v) = preset.template {
        config.template = Some(v.clone());
    }
    if let Some(ref v) = preset.font {
        config.font = Some(v.clone());
    }
    if let Some(v) = preset.font_size {
        config.font_size = Some(v);
    }
    if let Some(v) = preset.font_weight {
        config.font_weight = v;
    }
    if let Some(v) = preset.font_style {
        config.font_style = v;
    }
    if let Some(v) = preset.letter_spacing {
        config.letter_spacing = v;
    }
    if let Some(v) = preset.position {
        config.position = v;
    }
    if let Some(v) = preset.rotation {
        config.rotation = v;
    }
    if let Some(v) = preset.margin {
        config.margin = v;
    }
    if let Some(v) = preset.scale {
        config.scale = v;
    }
    if let Some(v) = preset.tile_spacing {
        config.tile_spacing = v;
    }
    if let Some(v) = preset.tile_rows {
        config.tile_rows = Some(v);
    }
    if let Some(v) = preset.tile_cols {
        config.tile_cols = Some(v);
    }
    if let Some(ref v) = preset.offset {
        config.offset = (v[0], v[1]);
    }
    if let Some(ref v) = preset.color {
        config.color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = preset.opacity {
        config.opacity = v;
    }
    if let Some(v) = preset.background {
        config.background = v;
    }
    if let Some(ref v) = preset.bg_color {
        config.bg_color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = preset.bg_opacity {
        config.bg_opacity = v;
    }
    if let Some(v) = preset.blend {
        config.blend = v;
    }
    if let Some(v) = preset.border {
        config.border = v;
    }
    if let Some(ref v) = preset.border_color {
        config.border_color = Some(parse_color(v).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(v) = preset.border_width {
        config.border_width = v;
    }
    if let Some(v) = preset.border_style {
        config.border_style = v;
    }
    if let Some(v) = preset.shadow {
        config.shadow = v;
    }
    if let Some(ref v) = preset.shadow_color {
        config.shadow_color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = preset.shadow_offset {
        config.shadow_offset = (v[0], v[1]);
    }
    if let Some(v) = preset.shadow_blur {
        config.shadow_blur = v;
    }
    if let Some(v) = preset.shadow_opacity {
        config.shadow_opacity = v;
    }
    if let Some(v) = preset.invert {
        config.invert = v;
    }
    if let Some(v) = preset.grayscale {
        config.grayscale = v;
    }
    if let Some(v) = preset.filigrane {
        config.filigrane = v;
    }
    if let Some(v) = preset.anti_ai {
        config.anti_ai = v;
    }
    if let Some(ref v) = preset.pages {
        config.pages = parse_page_range(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = preset.skip_pages {
        config.skip_pages = Some(parse_page_range(v).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(ref v) = preset.layer_name {
        config.layer_name = v.clone();
    }
    if let Some(v) = preset.flatten {
        config.flatten = v;
    }
    if let Some(v) = preset.behind {
        config.behind = v;
    }
    if let Some(v) = preset.quality {
        config.quality = v;
    }
    if let Some(v) = preset.dpi {
        config.dpi = v;
    }
    if let Some(v) = preset.strip_metadata {
        config.strip_metadata = v;
    }
    if let Some(v) = preset.png_compression {
        config.png_compression = v;
    }
    if let Some(ref v) = preset.color_profile {
        config.color_profile = Some(v.clone());
    }

    Ok(())
}

fn merge_cli_args(config: &mut WatermarkConfig, args: &CliArgs) -> Result<()> {
    // Input is required (validated by clap unless list_presets/show_config)
    if let Some(ref input) = args.input {
        config.input = input.clone();
    }

    if let Some(ref v) = args.output {
        config.output = Some(v.clone());
    }
    if let Some(ref v) = args.suffix {
        config.suffix = Some(v.clone());
    }
    if args.recursive {
        config.recursive = true;
    }
    if let Some(v) = args.jobs {
        config.jobs = v;
    }
    if args.overwrite {
        config.overwrite = true;
    }
    if args.dry_run {
        config.dry_run = true;
    }

    // watermark_type always has a clap default, so we always apply it.
    // However, if a preset or TOML already set a different type and the user
    // did not explicitly pass --type, we should not clobber it.  Clap does not
    // distinguish "default" from "explicitly given" for non-Option types, so
    // we rely on the fact that CliArgs.watermark_type always has a value.
    // Because CLI is highest priority we always set it here.
    config.watermark_type = args.watermark_type;

    if let Some(ref v) = args.main_text {
        config.main_text = v.clone();
    }
    if let Some(ref v) = args.secondary_text {
        config.secondary_text = v.clone();
    }
    if let Some(ref v) = args.image {
        config.image_path = Some(v.clone());
    }
    if let Some(ref v) = args.qr_data {
        config.qr_data = Some(v.clone());
    }
    if let Some(ref v) = args.template {
        config.template = Some(v.clone());
    }
    if let Some(ref v) = args.font {
        config.font = Some(v.clone());
    }
    if let Some(v) = args.font_size {
        config.font_size = Some(v);
    }
    if let Some(v) = args.font_weight {
        config.font_weight = v;
    }
    if let Some(v) = args.font_style {
        config.font_style = v;
    }
    if let Some(v) = args.letter_spacing {
        config.letter_spacing = v;
    }
    if let Some(v) = args.position {
        config.position = v;
    }
    if let Some(v) = args.rotation {
        config.rotation = v;
    }
    if let Some(v) = args.margin {
        config.margin = v;
    }
    if let Some(v) = args.scale {
        config.scale = v;
    }
    if let Some(v) = args.tile_spacing {
        config.tile_spacing = v;
    }
    if let Some(v) = args.tile_rows {
        config.tile_rows = Some(v);
    }
    if let Some(v) = args.tile_cols {
        config.tile_cols = Some(v);
    }
    if let Some(ref v) = args.offset {
        config.offset = parse_offset(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = args.color {
        config.color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = args.opacity {
        config.opacity = v;
    }
    if let Some(v) = args.background {
        config.background = v;
    }
    if let Some(ref v) = args.bg_color {
        config.bg_color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = args.bg_opacity {
        config.bg_opacity = v;
    }
    if let Some(v) = args.blend {
        config.blend = v;
    }
    if args.border {
        config.border = true;
    }
    if let Some(ref v) = args.border_color {
        config.border_color = Some(parse_color(v).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(v) = args.border_width {
        config.border_width = v;
    }
    if let Some(v) = args.border_style {
        config.border_style = v;
    }
    if args.shadow {
        config.shadow = true;
    }
    if let Some(ref v) = args.shadow_color {
        config.shadow_color = parse_color(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = args.shadow_offset {
        config.shadow_offset = parse_offset(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(v) = args.shadow_blur {
        config.shadow_blur = v;
    }
    if let Some(v) = args.shadow_opacity {
        config.shadow_opacity = v;
    }
    if args.invert {
        config.invert = true;
    }
    if args.grayscale {
        config.grayscale = true;
    }
    if let Some(v) = args.filigrane {
        config.filigrane = v;
    }
    if args.no_anti_ai {
        config.anti_ai = false;
    }
    if let Some(ref v) = args.pages {
        config.pages = parse_page_range(v).map_err(|e| anyhow::anyhow!(e))?;
    }
    if let Some(ref v) = args.skip_pages {
        config.skip_pages = Some(parse_page_range(v).map_err(|e| anyhow::anyhow!(e))?);
    }
    if let Some(ref v) = args.layer_name {
        config.layer_name = v.clone();
    }
    if args.no_flatten {
        config.flatten = false;
    }
    if args.behind {
        config.behind = true;
    }
    if let Some(v) = args.quality {
        config.quality = v;
    }
    if let Some(v) = args.dpi {
        config.dpi = v;
    }
    if args.strip_metadata {
        config.strip_metadata = true;
    }
    if let Some(v) = args.png_compression {
        config.png_compression = v;
    }
    if let Some(ref v) = args.color_profile {
        config.color_profile = Some(v.clone());
    }

    Ok(())
}

/// Persist the current CLI flags as a named preset in the config file.
fn save_preset(args: &CliArgs, preset_name: &str) -> Result<()> {
    let config_path = match args.config {
        Some(ref p) => p.clone(),
        None => default_config_path()?,
    };

    // Load existing config or start fresh
    let mut toml_config = if config_path.exists() {
        load_config(&config_path)?
    } else {
        TomlConfig::default()
    };

    // Build a PresetConfig from the current CLI args
    let preset = preset_from_cli(args);
    toml_config
        .preset
        .insert(preset_name.to_string(), preset);

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let serialized = toml::to_string_pretty(&toml_config)
        .context("Failed to serialize config to TOML")?;
    std::fs::write(&config_path, serialized)
        .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;

    eprintln!("Preset '{preset_name}' saved to {}", config_path.display());
    Ok(())
}

/// Convert CLI args into a `PresetConfig`, capturing only the explicitly-set values.
fn preset_from_cli(args: &CliArgs) -> PresetConfig {
    PresetConfig {
        output: args.output.clone(),
        suffix: args.suffix.clone(),
        recursive: if args.recursive { Some(true) } else { None },
        jobs: args.jobs,
        overwrite: if args.overwrite { Some(true) } else { None },
        watermark_type: Some(args.watermark_type),
        main_text: args.main_text.clone(),
        secondary_text: args.secondary_text.clone(),
        image_path: args.image.clone(),
        qr_data: args.qr_data.clone(),
        template: args.template.clone(),
        font: args.font.clone(),
        font_size: args.font_size,
        font_weight: args.font_weight,
        font_style: args.font_style,
        letter_spacing: args.letter_spacing,
        position: args.position,
        rotation: args.rotation,
        margin: args.margin,
        scale: args.scale,
        tile_spacing: args.tile_spacing,
        tile_rows: args.tile_rows,
        tile_cols: args.tile_cols,
        offset: args.offset.as_ref().and_then(|s| {
            parse_offset(s).ok().map(|(x, y)| [x, y])
        }),
        color: args.color.clone(),
        opacity: args.opacity,
        background: args.background,
        bg_color: args.bg_color.clone(),
        bg_opacity: args.bg_opacity,
        blend: args.blend,
        border: if args.border { Some(true) } else { None },
        border_color: args.border_color.clone(),
        border_width: args.border_width,
        border_style: args.border_style,
        shadow: if args.shadow { Some(true) } else { None },
        shadow_color: args.shadow_color.clone(),
        shadow_offset: args.shadow_offset.as_ref().and_then(|s| {
            parse_offset(s).ok().map(|(x, y)| [x, y])
        }),
        shadow_blur: args.shadow_blur,
        shadow_opacity: args.shadow_opacity,
        invert: if args.invert { Some(true) } else { None },
        grayscale: if args.grayscale { Some(true) } else { None },
        filigrane: args.filigrane,
        anti_ai: if args.no_anti_ai { Some(false) } else { None },
        pages: args.pages.clone(),
        skip_pages: args.skip_pages.clone(),
        layer_name: args.layer_name.clone(),
        flatten: if args.no_flatten { Some(false) } else { None },
        behind: if args.behind { Some(true) } else { None },
        quality: args.quality,
        dpi: args.dpi,
        strip_metadata: if args.strip_metadata { Some(true) } else { None },
        png_compression: args.png_compression,
        color_profile: args.color_profile.clone(),
    }
}
