use clap::Parser;
use std::path::PathBuf;

use crate::watermark::WatermarkType;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Center,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Tile,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    SoftLight,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Thin,
    Light,
    Regular,
    Bold,
    Black,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundPattern {
    None,
    Grid,
    Dots,
    Lines,
    Crosshatch,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum FiligraneStyle {
    Full,
    Guilloche,
    Rosette,
    Crosshatch,
    Border,
    Lissajous,
    Moire,
    Spiral,
    Mesh,
    Plume,
    Constellation,
    Ripple,
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
    after_help = after_help_text(),
)]
pub struct CliArgs {
    // ── Input / Output ──
    /// Input file or folder
    #[arg(required_unless_present_any = ["list_presets", "show_config"], help_heading = "Input / Output")]
    pub input: Option<PathBuf>,

    /// Output path (.jpg, .png, .webp, .tiff, .pdf)
    #[arg(short, long, value_name = "FILE", help_heading = "Input / Output")]
    pub output: Option<PathBuf>,

    /// Output filename suffix: {name}-{suffix}.ext
    #[arg(
        short = 'S',
        long,
        value_name = "TEXT",
        help_heading = "Input / Output"
    )]
    pub suffix: Option<String>,

    /// Recurse into folders
    #[arg(short = 'R', long, help_heading = "Input / Output")]
    pub recursive: bool,

    /// Parallel workers (default: CPU count)
    #[arg(short, long, value_name = "N", help_heading = "Input / Output")]
    pub jobs: Option<usize>,

    /// Overwrite existing outputs without prompting
    #[arg(long, help_heading = "Input / Output")]
    pub overwrite: bool,

    /// Preview without writing files
    #[arg(short = 'n', long, help_heading = "Input / Output")]
    pub dry_run: bool,

    // ── Watermark Type ──
    /// Watermark style (list below)
    #[arg(
        short = 't',
        long = "type",
        value_enum,
        default_value = "diagonal",
        value_name = "TYPE",
        hide_possible_values = true,
        help_heading = "Watermark Type"
    )]
    pub watermark_type: WatermarkType,

    // ── Content & Templates ──
    /// Primary text (default: "firemark")
    #[arg(short = 'm', long, value_name = "TEXT", help_heading = "Content")]
    pub main_text: Option<String>,

    /// Secondary text below/around the main text (default: {timestamp})
    #[arg(short = 's', long, value_name = "TEXT", help_heading = "Content")]
    pub secondary_text: Option<String>,

    /// Overlay an image file as/with the watermark
    #[arg(short = 'I', long, value_name = "FILE", help_heading = "Content")]
    pub image: Option<PathBuf>,

    /// Encode data as a QR code overlay
    #[arg(long, value_name = "TEXT", help_heading = "Content")]
    pub qr_data: Option<String>,

    /// QR position, same values as -p (default: center)
    #[arg(
        long,
        value_enum,
        value_name = "POS",
        hide_possible_values = true,
        help_heading = "Content"
    )]
    pub qr_code_position: Option<Position>,

    /// QR size in pixels (default: auto)
    #[arg(long, value_name = "PX", help_heading = "Content")]
    pub qr_code_size: Option<u32>,

    /// Full text template using {variables}
    #[arg(long, value_name = "TEXT", help_heading = "Content")]
    pub template: Option<String>,

    // ── Typography ──
    /// Font name or .ttf/.otf path
    #[arg(short, long, value_name = "NAME", help_heading = "Typography")]
    pub font: Option<String>,

    /// Font size in points (default: auto)
    #[arg(long, value_name = "PT", help_heading = "Typography")]
    pub font_size: Option<f32>,

    /// Weight: thin|light|regular|bold|black
    #[arg(
        long,
        value_enum,
        value_name = "W",
        hide_possible_values = true,
        help_heading = "Typography"
    )]
    pub font_weight: Option<FontWeight>,

    /// Style: normal|italic
    #[arg(
        long,
        value_enum,
        value_name = "S",
        hide_possible_values = true,
        help_heading = "Typography"
    )]
    pub font_style: Option<FontStyle>,

    /// Extra letter spacing in pixels
    #[arg(long, value_name = "PX", help_heading = "Typography")]
    pub letter_spacing: Option<f32>,

    // ── Position & Layout ──
    /// center|top-left|top-right|bottom-left|bottom-right|tile
    #[arg(
        short,
        long,
        value_enum,
        value_name = "POS",
        hide_possible_values = true,
        help_heading = "Position & Layout"
    )]
    pub position: Option<Position>,

    /// Rotation in degrees (default: -45)
    #[arg(
        short,
        long,
        allow_hyphen_values = true,
        value_name = "DEG",
        help_heading = "Position & Layout"
    )]
    pub rotation: Option<f32>,

    /// Edge margin in pixels (default: 20)
    #[arg(long, value_name = "PX", help_heading = "Position & Layout")]
    pub margin: Option<u32>,

    /// Size relative to canvas, 0.0-1.0 (default: 0.4)
    #[arg(long, value_name = "0-1", help_heading = "Position & Layout")]
    pub scale: Option<f32>,

    /// Gap between tiles in pixels (default: 80)
    #[arg(long, value_name = "PX", help_heading = "Position & Layout")]
    pub tile_spacing: Option<u32>,

    /// Fixed tile row count
    #[arg(long, value_name = "N", help_heading = "Position & Layout")]
    pub tile_rows: Option<u32>,

    /// Fixed tile column count
    #[arg(long, value_name = "N", help_heading = "Position & Layout")]
    pub tile_cols: Option<u32>,

    /// Pixel offset from anchor, e.g. 10,-5
    #[arg(long, value_name = "X,Y", help_heading = "Position & Layout")]
    pub offset: Option<String>,

    // ── Style & Appearance ──
    /// Watermark color, named or #RRGGBB (default: blue)
    #[arg(short, long, value_name = "COLOR", help_heading = "Style & Appearance")]
    pub color: Option<String>,

    /// Opacity, 0.0-1.0 (default: 0.5)
    #[arg(
        short = 'O',
        long,
        value_name = "0-1",
        help_heading = "Style & Appearance"
    )]
    pub opacity: Option<f32>,

    /// Backdrop pattern: none|grid|dots|lines|crosshatch
    #[arg(
        short,
        long,
        value_enum,
        value_name = "PAT",
        hide_possible_values = true,
        help_heading = "Style & Appearance"
    )]
    pub background: Option<BackgroundPattern>,

    /// Backdrop color (default: #CCCCCC)
    #[arg(long, value_name = "COLOR", help_heading = "Style & Appearance")]
    pub bg_color: Option<String>,

    /// Backdrop opacity, 0.0-1.0 (default: 0.15)
    #[arg(long, value_name = "0-1", help_heading = "Style & Appearance")]
    pub bg_opacity: Option<f32>,

    /// Blend: normal|multiply|screen|overlay|soft-light
    #[arg(
        long,
        value_enum,
        value_name = "MODE",
        hide_possible_values = true,
        help_heading = "Style & Appearance"
    )]
    pub blend: Option<BlendMode>,

    /// Draw a border around the watermark
    #[arg(long, help_heading = "Style & Appearance")]
    pub border: bool,

    /// Border color (default: same as --color)
    #[arg(long, value_name = "COLOR", help_heading = "Style & Appearance")]
    pub border_color: Option<String>,

    /// Border width in pixels (default: 1)
    #[arg(long, value_name = "PX", help_heading = "Style & Appearance")]
    pub border_width: Option<u32>,

    /// Border style: solid|dashed|dotted
    #[arg(
        long,
        value_enum,
        value_name = "STYLE",
        hide_possible_values = true,
        help_heading = "Style & Appearance"
    )]
    pub border_style: Option<BorderStyle>,

    /// Add a drop shadow
    #[arg(long, help_heading = "Style & Appearance")]
    pub shadow: bool,

    /// Shadow color (default: #000000)
    #[arg(long, value_name = "COLOR", help_heading = "Style & Appearance")]
    pub shadow_color: Option<String>,

    /// Shadow offset in pixels, e.g. 2,2
    #[arg(long, value_name = "X,Y", help_heading = "Style & Appearance")]
    pub shadow_offset: Option<String>,

    /// Shadow blur radius in pixels (default: 4)
    #[arg(long, value_name = "PX", help_heading = "Style & Appearance")]
    pub shadow_blur: Option<u32>,

    /// Shadow opacity, 0.0-1.0 (default: 0.4)
    #[arg(long, value_name = "0-1", help_heading = "Style & Appearance")]
    pub shadow_opacity: Option<f32>,

    /// Render watermark in inverted color
    #[arg(long, help_heading = "Style & Appearance")]
    pub invert: bool,

    /// Force grayscale rendering
    #[arg(long, help_heading = "Style & Appearance")]
    pub grayscale: bool,

    // ── Security ──
    /// Filigrane pattern (default: guilloche, list below)
    #[arg(
        long,
        value_enum,
        value_name = "STYLE",
        hide_possible_values = true,
        help_heading = "Security"
    )]
    pub filigrane: Option<FiligraneStyle>,

    /// Disable anti-AI hardening (adversarial text + entangle strokes)
    #[arg(long, help_heading = "Security")]
    pub no_anti_ai: bool,

    // ── PDF-specific ──
    /// Pages to watermark, e.g. 1,3-5,8 (default: all)
    #[arg(long, value_name = "RANGE", help_heading = "PDF")]
    pub pages: Option<String>,

    /// Pages to skip, same range syntax
    #[arg(long, value_name = "RANGE", help_heading = "PDF")]
    pub skip_pages: Option<String>,

    /// Layer name (default: "Watermark")
    #[arg(long, value_name = "NAME", help_heading = "PDF")]
    pub layer_name: Option<String>,

    /// Keep watermark on a separate layer (flattened by default)
    #[arg(long, help_heading = "PDF")]
    pub no_flatten: bool,

    /// Disable invisible copy-paste poisoning (on by default)
    #[arg(long, help_heading = "PDF")]
    pub no_copy_poison: bool,

    /// Place watermark behind existing content
    #[arg(long, help_heading = "PDF")]
    pub behind: bool,

    // ── Output Quality ──
    /// JPEG quality, 1-100 (default: 90)
    #[arg(short, long, value_name = "1-100", help_heading = "Output Quality")]
    pub quality: Option<u8>,

    /// Output DPI (default: 150)
    #[arg(long, value_name = "N", help_heading = "Output Quality")]
    pub dpi: Option<u32>,

    /// Strip EXIF/XMP metadata from output
    #[arg(long, help_heading = "Output Quality")]
    pub strip_metadata: bool,

    /// PNG compression, 0-9 (default: 6)
    #[arg(long, value_name = "0-9", help_heading = "Output Quality")]
    pub png_compression: Option<u8>,

    /// Embed an ICC color profile
    #[arg(long, value_name = "FILE", help_heading = "Output Quality")]
    pub color_profile: Option<PathBuf>,

    // ── Config & Presets ──
    /// Load options from a TOML config file
    #[arg(long, value_name = "FILE", help_heading = "Config & Presets")]
    pub config: Option<PathBuf>,

    /// Use a named preset from the config file
    #[arg(long, value_name = "NAME", help_heading = "Config & Presets")]
    pub preset: Option<String>,

    /// Save current flags as a named preset
    #[arg(long, value_name = "NAME", help_heading = "Config & Presets")]
    pub save_preset: Option<String>,

    /// List all available presets
    #[arg(long, help_heading = "Config & Presets")]
    pub list_presets: bool,

    /// Print the resolved config and exit
    #[arg(long, help_heading = "Config & Presets")]
    pub show_config: bool,

    // ── General ──
    /// Detailed per-file progress
    #[arg(short, long, help_heading = "General")]
    pub verbose: bool,

    /// Only errors
    #[arg(long, conflicts_with = "verbose", help_heading = "General")]
    pub quiet: bool,

    /// Write log output to a file
    #[arg(long, value_name = "FILE", help_heading = "General")]
    pub log: Option<PathBuf>,

    /// Disable colored terminal output
    #[arg(long, help_heading = "General")]
    pub no_color: bool,
}

fn cli_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Effects, Style};
    clap::builder::Styles::styled()
        .header(
            Style::new()
                .fg_color(Some(AnsiColor::Yellow.into()))
                .effects(Effects::BOLD),
        )
        .usage(
            Style::new()
                .fg_color(Some(AnsiColor::Yellow.into()))
                .effects(Effects::BOLD),
        )
        .literal(Style::new().fg_color(Some(AnsiColor::Green.into())))
        .placeholder(Style::new().fg_color(Some(AnsiColor::Cyan.into())))
        .valid(Style::new().fg_color(Some(AnsiColor::Green.into())))
        .invalid(
            Style::new()
                .fg_color(Some(AnsiColor::Red.into()))
                .effects(Effects::BOLD),
        )
        .error(
            Style::new()
                .fg_color(Some(AnsiColor::Red.into()))
                .effects(Effects::BOLD),
        )
}

/// Compact two-column reference for the value enums whose listings are hidden
/// from the options table to keep it one line per flag.
fn after_help_text() -> String {
    const TYPES: &[(&str, &str)] = &[
        ("diagonal", "Diagonal text grid"),
        ("stamp", "Rubber stamp, double border"),
        ("stencil", "Stencil lettering"),
        ("typewriter", "Typewriter text page"),
        ("handwritten", "Signature with underline"),
        ("redacted", "Black redaction bars"),
        ("badge", "Security shield emblem"),
        ("ribbon", "Corner ribbon banner"),
        ("seal", "Circular notary seal"),
        ("frame", "Decorative border frame"),
        ("tile", "Dense text tile grid"),
        ("mosaic", "Scattered text mosaic"),
        ("weave", "Diagonal weave pattern"),
        ("ghost", "Subtle embossed text"),
        ("watercolor", "Soft watercolour wash"),
        ("noise", "Distressed text + noise"),
        ("halftone", "Halftone dot text"),
    ];
    const FILIGRANES: &[(&str, &str)] = &[
        ("full", "All patterns combined"),
        ("guilloche", "Banknote wave bands"),
        ("rosette", "Spirograph rosettes"),
        ("crosshatch", "Diamond lattice grid"),
        ("border", "Wavy security border"),
        ("lissajous", "Lissajous figures"),
        ("moire", "Circle interference"),
        ("spiral", "Archimedean spiral"),
        ("mesh", "Honeycomb mesh"),
        ("plume", "Feather-like plumes"),
        ("constellation", "Star node web"),
        ("ripple", "Elliptical wave fronts"),
        ("none", "Disabled"),
    ];

    let two_columns = |items: &[(&str, &str)]| -> String {
        items
            .chunks(2)
            .map(|pair| {
                let mut line = String::from("  ");
                for (name, desc) in pair {
                    line.push_str(&format!("\x1b[32m{name:<14}\x1b[0m{desc:<31}"));
                }
                line.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "\x1b[1;33mWatermark types (-t):\x1b[0m\n{}\n\n\x1b[1;33mFiligrane styles (--filigrane):\x1b[0m\n{}",
        two_columns(TYPES),
        two_columns(FILIGRANES),
    )
}
