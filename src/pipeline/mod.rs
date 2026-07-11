pub mod batch;
pub mod image_pipeline;
pub mod io;
pub mod pdf_pipeline;

use crate::cli::args::CliArgs;
use crate::config::types::WatermarkConfig;
use crate::error::FiremarkError;

pub fn dispatch(config: &WatermarkConfig, args: &CliArgs) -> anyhow::Result<()> {
    let input = &config.input;

    if input.is_dir() {
        batch::process_batch(config, args)?;
    } else if input.is_file() {
        process_single_file(config, args)?;
    } else {
        return Err(FiremarkError::InvalidArgument(format!(
            "Input path does not exist: {}",
            input.display()
        ))
        .into());
    }

    Ok(())
}

pub fn process_single_file(config: &WatermarkConfig, args: &CliArgs) -> anyhow::Result<()> {
    let input = &config.input;
    let ext = io::detect_format(input)?;

    match ext {
        io::FileFormat::Jpeg
        | io::FileFormat::Png
        | io::FileFormat::WebP
        | io::FileFormat::Tiff => {
            image_pipeline::process_image(config, args)?;
        }
        io::FileFormat::Pdf => {
            pdf_pipeline::process_pdf(config, args)?;
        }
    }

    Ok(())
}
