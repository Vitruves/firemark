use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use anyhow::Context;
use log::{error, info, warn};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::cli::args::CliArgs;
use crate::config::types::WatermarkConfig;
use crate::pipeline::io::{is_supported, resolve_output_path};
use crate::pipeline::process_single_file;

/// Process all supported files in a directory, optionally recursing into
/// subdirectories.
///
/// Files are processed in parallel using a rayon thread pool whose size is
/// governed by `config.jobs`.  Progress is reported through an `indicatif`
/// progress bar.
pub fn process_batch(config: &WatermarkConfig, args: &CliArgs) -> anyhow::Result<()> {
    let input_dir = &config.input;

    if !input_dir.is_dir() {
        anyhow::bail!("Batch input is not a directory: {}", input_dir.display());
    }

    // Collect all supported files, skipping previously watermarked outputs.
    let suffix = config.suffix.as_deref().unwrap_or("watermarked");
    let max_depth = if config.recursive { usize::MAX } else { 1 };
    let files: Vec<_> = WalkDir::new(input_dir)
        .max_depth(max_depth)
        .follow_links(true)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| is_supported(entry.path()))
        .filter(|entry| {
            let stem = entry
                .path()
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy();
            !stem.ends_with(suffix)
        })
        .map(|entry| entry.into_path())
        .collect();

    if files.is_empty() {
        warn!("No supported files found in {}", input_dir.display());
        return Ok(());
    }

    info!(
        "Found {} file(s) to process in {}",
        files.len(),
        input_dir.display()
    );

    // Dry-run: just list files.
    if config.dry_run {
        info!("[dry-run] Would process {} file(s):", files.len());
        for f in &files {
            let out = resolve_output_path(
                f,
                None, // batch mode always generates per-file output
                config.suffix.as_deref(),
            );
            info!("  {} -> {}", f.display(), out.display());
        }
        return Ok(());
    }

    // Set up rayon thread pool.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.jobs)
        .build()
        .context("Failed to create thread pool")?;

    let total = files.len();
    let processed = AtomicUsize::new(0);
    let error_count = AtomicUsize::new(0);
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pool.install(|| {
        files.par_iter().for_each(|file_path| {
            let mut file_config = config.clone();
            file_config.input = file_path.clone();
            let out_path = resolve_output_path(file_path, None, config.suffix.as_deref());
            file_config.output = Some(out_path.clone());

            match process_single_file(&file_config, args) {
                Ok(()) => {
                    let done = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    info!(
                        "[{done}/{total}] {} -> {}",
                        file_path.display(),
                        out_path.display()
                    );
                }
                Err(e) => {
                    let done = processed.fetch_add(1, Ordering::Relaxed) + 1;
                    let msg = format!("{}: {e:#}", file_path.display());
                    error!("[{done}/{total}] {msg}");
                    error_count.fetch_add(1, Ordering::Relaxed);
                    if let Ok(mut errs) = errors.lock() {
                        errs.push(msg);
                    }
                }
            }
        });
    });

    // Report summary.
    let err_total = error_count.load(Ordering::Relaxed);
    let success = files.len() - err_total;

    if err_total > 0 {
        warn!("{success} succeeded, {err_total} failed");
        if let Ok(errs) = errors.lock() {
            for e in errs.iter() {
                warn!("  - {e}");
            }
        }
    } else {
        info!("All {success} file(s) processed successfully");
    }

    Ok(())
}
