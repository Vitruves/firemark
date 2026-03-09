use clap::Parser;
use std::path::PathBuf;

use crate::watermark::WatermarkType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Tile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Thin,
    Light,
    Regular,
    Bold,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundPattern {
    None,
    Grid,
    Dots,
    Lines,
    Crosshatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FiligraneStyle {
    /// All security patterns combined
    Full,
    /// Sinusoidal wave envelope bands (banknote-style)
    Guilloche,
    /// Central spirograph + corner rose curves
    Rosette,
    /// Fine diagonal diamond lattice grid
    Crosshatch,
    /// Wavy nested security border frame
    Border,
    /// Parametric Lissajous figure overlay
    Lissajous,
    /// Concentric circle interference pattern
    Moire,
    /// Archimedean spiral radiating from centre
    Spiral,
    /// Fine hexagonal honeycomb mesh
    Mesh,
    /// Disable filigrane overlay
    None,
}

/// A fast, flexible watermarking tool for images and PDFs
#[derive(Parser, Debug)]
#[command(
    name = "firemark",
    version,
    about = "A fast, flexible watermarking tool for images and PDFs",
    long_about = None,
    help_template = "\
{before-help}\x1b[1;36m{name}\x1b[0m {version}
{about}

\x1b[1;33mUsage:\x1b[0m {usage}

{all-args}{after-help}",
    term_width = 100,
    styles = cli_styles(),
)]
pub struct CliArgs {
    // ── Input / Output ──
    /// Input file or folder
    #[arg(required_unless_present_any = ["list_presets", "show_config"], help_heading = "Input / Output")]
    pub input: Option<PathBuf>,

    /// Output file path (.jpg, .jpeg, .png, .pdf)
    #[arg(short, long, help_heading = "Input / Output")]
    pub output: Option<PathBuf>,

    /// Append suffix to output filename: {name}-{suffix}.ext
    #[arg(short = 'S', long, help_heading = "Input / Output")]
    pub suffix: Option<String>,

    /// Process folders recursively
    #[arg(short = 'R', long, help_heading = "Input / Output")]
    pub recursive: bool,

    /// Number of parallel worker threads (default: CPU count)
    #[arg(short, long, help_heading = "Input / Output")]
    pub jobs: Option<usize>,

    /// Overwrite existing output files without prompting
    #[arg(long, help_heading = "Input / Output")]
    pub overwrite: bool,

    /// Preview operations without writing any files
    #[arg(short = 'n', long, help_heading = "Input / Output")]
    pub dry_run: bool,

    // ── Watermark Type ──
    /// Visual design style of the watermark
    #[arg(short = 't', long = "type", value_enum, default_value = "diagonal", help_heading = "Watermark Type")]
    pub watermark_type: WatermarkType,

    // ── Content & Templates ──
    /// Primary watermark text (default: "firemark")
    #[arg(short = 'm', long, help_heading = "Content")]
    pub main_text: Option<String>,

    /// Secondary text below or around the main text
    #[arg(short = 's', long, help_heading = "Content")]
    pub secondary_text: Option<String>,

    /// Image file to use as or combine with watermark
    #[arg(short = 'I', long, help_heading = "Content")]
    pub image: Option<PathBuf>,

    /// Data to encode as a QR code watermark
    #[arg(long, help_heading = "Content")]
    pub qr_data: Option<String>,

    /// Full text template using {variables}
    #[arg(long, help_heading = "Content")]
    pub template: Option<String>,

    // ── Typography ──
    /// Font name or path to a .ttf/.otf file
    #[arg(short, long, help_heading = "Typography")]
    pub font: Option<String>,

    /// Font size in points (default: auto-scaled)
    #[arg(long, help_heading = "Typography")]
    pub font_size: Option<f32>,

    /// Font weight
    #[arg(long, value_enum, help_heading = "Typography")]
    pub font_weight: Option<FontWeight>,

    /// Font style
    #[arg(long, value_enum, help_heading = "Typography")]
    pub font_style: Option<FontStyle>,

    /// Extra spacing between characters in pixels
    #[arg(long, help_heading = "Typography")]
    pub letter_spacing: Option<f32>,

    // ── Position & Layout ──
    /// Placement position
    #[arg(short, long, value_enum, help_heading = "Position & Layout")]
    pub position: Option<Position>,

    /// Rotation angle in degrees (default: -45)
    #[arg(short, long, allow_hyphen_values = true, help_heading = "Position & Layout")]
    pub rotation: Option<f32>,

    /// Edge margin in pixels (default: 20)
    #[arg(long, help_heading = "Position & Layout")]
    pub margin: Option<u32>,

    /// Watermark size relative to canvas width, 0.0-1.0 (default: 0.4)
    #[arg(long, help_heading = "Position & Layout")]
    pub scale: Option<f32>,

    /// Gap between tiles in pixels (default: 80)
    #[arg(long, help_heading = "Position & Layout")]
    pub tile_spacing: Option<u32>,

    /// Force a fixed number of tile rows
    #[arg(long, help_heading = "Position & Layout")]
    pub tile_rows: Option<u32>,

    /// Force a fixed number of tile columns
    #[arg(long, help_heading = "Position & Layout")]
    pub tile_cols: Option<u32>,

    /// Manual pixel offset from anchor, e.g. 10,-5
    #[arg(long, help_heading = "Position & Layout")]
    pub offset: Option<String>,

    // ── Style & Appearance ──
    /// Watermark color — named or hex #RRGGBB (default: #808080)
    #[arg(short, long, help_heading = "Style & Appearance")]
    pub color: Option<String>,

    /// Overall watermark opacity, 0.0-1.0 (default: 0.5)
    #[arg(short = 'O', long, help_heading = "Style & Appearance")]
    pub opacity: Option<f32>,

    /// Background pattern behind watermark
    #[arg(short, long, value_enum, help_heading = "Style & Appearance")]
    pub background: Option<BackgroundPattern>,

    /// Background pattern color (default: #CCCCCC)
    #[arg(long, help_heading = "Style & Appearance")]
    pub bg_color: Option<String>,

    /// Background pattern opacity, 0.0-1.0 (default: 0.15)
    #[arg(long, help_heading = "Style & Appearance")]
    pub bg_opacity: Option<f32>,

    /// Blend mode
    #[arg(long, value_enum, help_heading = "Style & Appearance")]
    pub blend: Option<BlendMode>,

    /// Draw a border around the watermark
    #[arg(long, help_heading = "Style & Appearance")]
    pub border: bool,

    /// Border color (default: same as --color)
    #[arg(long, help_heading = "Style & Appearance")]
    pub border_color: Option<String>,

    /// Border stroke width in pixels (default: 1)
    #[arg(long, help_heading = "Style & Appearance")]
    pub border_width: Option<u32>,

    /// Border line style
    #[arg(long, value_enum, help_heading = "Style & Appearance")]
    pub border_style: Option<BorderStyle>,

    /// Add a drop shadow
    #[arg(long, help_heading = "Style & Appearance")]
    pub shadow: bool,

    /// Shadow color (default: #000000)
    #[arg(long, help_heading = "Style & Appearance")]
    pub shadow_color: Option<String>,

    /// Shadow offset in pixels, e.g. 2,2
    #[arg(long, help_heading = "Style & Appearance")]
    pub shadow_offset: Option<String>,

    /// Shadow blur radius in pixels (default: 4)
    #[arg(long, help_heading = "Style & Appearance")]
    pub shadow_blur: Option<u32>,

    /// Shadow opacity, 0.0-1.0 (default: 0.4)
    #[arg(long, help_heading = "Style & Appearance")]
    pub shadow_opacity: Option<f32>,

    /// Render watermark in inverted color
    #[arg(long, help_heading = "Style & Appearance")]
    pub invert: bool,

    /// Force grayscale rendering
    #[arg(long, help_heading = "Style & Appearance")]
    pub grayscale: bool,

    /// Cryptographic filigrane security overlay style (default: guilloche)
    #[arg(long, value_enum, help_heading = "Style & Appearance")]
    pub filigrane: Option<FiligraneStyle>,

    // ── Anti-AI ──
    /// Disable adversarial prompt text that deters AI-based watermark removal (on by default)
    #[arg(long, help_heading = "Style & Appearance")]
    pub no_anti_ai: bool,

    // ── PDF-specific ──
    /// Pages to watermark — e.g. 1,3-5,8 or "all" (default: all)
    #[arg(long, help_heading = "PDF")]
    pub pages: Option<String>,

    /// Pages to skip, same range syntax
    #[arg(long, help_heading = "PDF")]
    pub skip_pages: Option<String>,

    /// PDF Optional Content Group layer name (default: "Watermark")
    #[arg(long, help_heading = "PDF")]
    pub layer_name: Option<String>,

    /// Disable layer flattening (flattened by default for security)
    #[arg(long, help_heading = "PDF")]
    pub no_flatten: bool,

    /// Place watermark behind existing content
    #[arg(long, help_heading = "PDF")]
    pub behind: bool,

    // ── Output Quality ──
    /// JPEG output quality, 1-100 (default: 90)
    #[arg(short, long, help_heading = "Output Quality")]
    pub quality: Option<u8>,

    /// Output DPI resolution (default: 150)
    #[arg(long, help_heading = "Output Quality")]
    pub dpi: Option<u32>,

    /// Strip EXIF and XMP metadata from output
    #[arg(long, help_heading = "Output Quality")]
    pub strip_metadata: bool,

    /// PNG compression level, 0-9 (default: 6)
    #[arg(long, help_heading = "Output Quality")]
    pub png_compression: Option<u8>,

    /// Embed an ICC color profile in the output
    #[arg(long, help_heading = "Output Quality")]
    pub color_profile: Option<PathBuf>,

    // ── Config & Presets ──
    /// Load options from a TOML configuration file
    #[arg(long, help_heading = "Config & Presets")]
    pub config: Option<PathBuf>,

    /// Use a named preset from the config file
    #[arg(long, help_heading = "Config & Presets")]
    pub preset: Option<String>,

    /// Save current flags as a named preset
    #[arg(long, help_heading = "Config & Presets")]
    pub save_preset: Option<String>,

    /// List all available presets
    #[arg(long, help_heading = "Config & Presets")]
    pub list_presets: bool,

    /// Print the resolved config and exit
    #[arg(long, help_heading = "Config & Presets")]
    pub show_config: bool,

    // ── General ──
    /// Print detailed per-file progress
    #[arg(short, long, help_heading = "General")]
    pub verbose: bool,

    /// Suppress all output except errors
    #[arg(long, conflicts_with = "verbose", help_heading = "General")]
    pub quiet: bool,

    /// Write log output to a file
    #[arg(long, help_heading = "General")]
    pub log: Option<PathBuf>,

    /// Disable colored terminal output
    #[arg(long, help_heading = "General")]
    pub no_color: bool,
}

fn cli_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Effects, Style};
    clap::builder::Styles::styled()
        .header(Style::new().fg_color(Some(AnsiColor::Yellow.into())).effects(Effects::BOLD))
        .usage(Style::new().fg_color(Some(AnsiColor::Yellow.into())).effects(Effects::BOLD))
        .literal(Style::new().fg_color(Some(AnsiColor::Green.into())))
        .placeholder(Style::new().fg_color(Some(AnsiColor::Cyan.into())))
        .valid(Style::new().fg_color(Some(AnsiColor::Green.into())))
        .invalid(Style::new().fg_color(Some(AnsiColor::Red.into())).effects(Effects::BOLD))
        .error(Style::new().fg_color(Some(AnsiColor::Red.into())).effects(Effects::BOLD))
}
