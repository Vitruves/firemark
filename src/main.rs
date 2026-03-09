use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use log::info;

use firemark::cli::args::CliArgs;
use firemark::config::merge::resolve_config;
use firemark::pipeline::dispatch;

fn main() -> Result<()> {
    let args = CliArgs::parse();

    let log_level = if args.quiet {
        "error"
    } else if args.verbose {
        "debug"
    } else {
        "info"
    };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(log_level))
        .format_timestamp(None)
        .init();

    // Non-blocking version check (silently skipped if offline).
    firemark::update::check_for_update();

    if args.list_presets {
        return firemark::config::loader::list_presets(&args.config);
    }

    if args.show_config {
        let config = resolve_config(&args)?;
        println!("{}", toml::to_string_pretty(&config)?);
        return Ok(());
    }

    let config = resolve_config(&args)?;

    if args.dry_run {
        println!("{}", "Dry run — no files will be written.".yellow());
    }

    info!("Processing: {}", config.input.display());
    dispatch(&config, &args)?;

    Ok(())
}
