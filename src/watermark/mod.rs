pub mod anti_ai;
pub mod background;
pub mod effect;
pub mod filigrane;
pub mod pattern;
pub mod perturb;
pub mod renderer;
pub mod shape;
pub mod text;


pub use renderer::WatermarkRenderer;

use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WatermarkType {
    /// Full-page repeating diagonal text grid
    Diagonal,
    /// Large rubber-stamp with double border
    Stamp,
    /// Full-width military stencil lettering
    Stencil,
    /// Full-page monospaced typewriter text
    Typewriter,
    /// Handwritten signature with underline
    Handwritten,
    /// Full-width black redaction bars
    Redacted,
    /// Security shield / badge emblem
    Badge,
    /// Diagonal corner ribbon banner
    Ribbon,
    /// Circular notary-style seal
    Seal,
    /// Full-page decorative border frame
    Frame,
    /// Dense uniform text tile grid
    Tile,
    /// Randomised scattered text mosaic
    Mosaic,
    /// Interlocking diagonal weave pattern
    Weave,
    /// Ultra-subtle embossed text pattern
    Ghost,
    /// Soft blurred watercolour wash
    Watercolor,
    /// Distressed text with pixel noise
    Noise,
    /// Text converted to halftone dot grid
    Halftone,
}

impl FromStr for WatermarkType {
    type Err = String;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "diagonal" => Ok(Self::Diagonal),
            "stamp" => Ok(Self::Stamp),
            "stencil" => Ok(Self::Stencil),
            "typewriter" => Ok(Self::Typewriter),
            "handwritten" => Ok(Self::Handwritten),
            "redacted" => Ok(Self::Redacted),
            "badge" => Ok(Self::Badge),
            "ribbon" => Ok(Self::Ribbon),
            "seal" => Ok(Self::Seal),
            "frame" => Ok(Self::Frame),
            "tile" => Ok(Self::Tile),
            "mosaic" => Ok(Self::Mosaic),
            "weave" => Ok(Self::Weave),
            "ghost" => Ok(Self::Ghost),
            "watercolor" => Ok(Self::Watercolor),
            "noise" => Ok(Self::Noise),
            "halftone" => Ok(Self::Halftone),
            _ => Err(format!("Unknown watermark type: {s}")),
        }
    }
}

impl std::fmt::Display for WatermarkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Diagonal => "diagonal",
            Self::Stamp => "stamp",
            Self::Stencil => "stencil",
            Self::Typewriter => "typewriter",
            Self::Handwritten => "handwritten",
            Self::Redacted => "redacted",
            Self::Badge => "badge",
            Self::Ribbon => "ribbon",
            Self::Seal => "seal",
            Self::Frame => "frame",
            Self::Tile => "tile",
            Self::Mosaic => "mosaic",
            Self::Weave => "weave",
            Self::Ghost => "ghost",
            Self::Watercolor => "watercolor",
            Self::Noise => "noise",
            Self::Halftone => "halftone",
        };
        write!(f, "{s}")
    }
}

pub fn create_renderer(wm_type: WatermarkType) -> Box<dyn WatermarkRenderer> {
    match wm_type {
        WatermarkType::Diagonal => Box::new(text::DiagonalRenderer),
        WatermarkType::Stamp => Box::new(text::StampRenderer),
        WatermarkType::Stencil => Box::new(text::StencilRenderer),
        WatermarkType::Typewriter => Box::new(text::TypewriterRenderer),
        WatermarkType::Handwritten => Box::new(text::HandwrittenRenderer),
        WatermarkType::Redacted => Box::new(text::RedactedRenderer),
        WatermarkType::Badge => Box::new(shape::BadgeRenderer),
        WatermarkType::Ribbon => Box::new(shape::RibbonRenderer),
        WatermarkType::Seal => Box::new(shape::SealRenderer),
        WatermarkType::Frame => Box::new(shape::FrameRenderer),
        WatermarkType::Tile => Box::new(pattern::TileRenderer),
        WatermarkType::Mosaic => Box::new(pattern::MosaicRenderer),
        WatermarkType::Weave => Box::new(pattern::WeaveRenderer),
        WatermarkType::Ghost => Box::new(effect::GhostRenderer),
        WatermarkType::Watercolor => Box::new(effect::WatercolorRenderer),
        WatermarkType::Noise => Box::new(effect::NoiseRenderer),
        WatermarkType::Halftone => Box::new(effect::HalftoneRenderer),
    }
}
