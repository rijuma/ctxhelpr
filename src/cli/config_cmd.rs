use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::Path;

use super::style;
use crate::config::{
    CONFIG_FILENAME, Config, ConfigError, GLOBAL_CONFIG_DIR, GLOBAL_CONFIG_FILENAME,
    global_config_path,
};

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Create a config template (.ctxhelpr.json locally, or global with --global)
    Init {
        /// Create global config at ~/.config/ctxhelpr/config.json instead of local
        #[arg(long, short)]
        global: bool,
    },
    /// Validate a config file (.ctxhelpr.json locally, or global with --global)
    Validate {
        /// Directory containing .ctxhelpr.json (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
        /// Validate the global config file instead of a local one
        #[arg(long, short)]
        global: bool,
    },
    /// Show resolved configuration (defaults merged with overrides)
    Show {
        /// Directory containing .ctxhelpr.json (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
    },
}

pub fn run(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommand::Init { global } => {
            if global {
                run_init_global()
            } else {
                run_init_local()
            }
        }
        ConfigCommand::Validate { path, global } => {
            if global {
                run_validate_global()
            } else {
                run_validate_local(path)
            }
        }
        ConfigCommand::Show { path } => run_show(path),
    }
}

fn run_init_local() -> Result<()> {
    let config_path = Path::new(CONFIG_FILENAME);
    if config_path.exists() {
        println!(
            "{}",
            style::warn(&format!(
                "{} already exists in the current directory.",
                CONFIG_FILENAME
            ))
        );
        return Ok(());
    }

    let template = serde_json::to_string_pretty(&Config::default())?;
    std::fs::write(config_path, format!("{template}\n"))?;
    println!(
        "{}",
        style::success(&format!("Created {}", config_path.display()))
    );
    Ok(())
}

fn run_init_global() -> Result<()> {
    let Some(config_path) = global_config_path() else {
        println!(
            "{}",
            style::error("Could not determine config directory for this platform.")
        );
        return Ok(());
    };

    if config_path.exists() {
        println!(
            "{}",
            style::warn(&format!("{} already exists.", config_path.display()))
        );
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let template = serde_json::to_string_pretty(&Config::default())?;
    std::fs::write(&config_path, format!("{template}\n"))?;
    println!(
        "{}",
        style::success(&format!("Created {}", config_path.display()))
    );
    Ok(())
}

fn run_validate_local(path: Option<String>) -> Result<()> {
    let dir = path.unwrap_or_else(|| ".".to_string());
    match Config::validate(&dir) {
        Ok(config) => {
            println!("{}\n", style::success("Valid"));
            print_config_summary(&config);
            Ok(())
        }
        Err(ConfigError::NotFound { .. }) => {
            println!("No {} found in {}", CONFIG_FILENAME, dir);
            Ok(())
        }
        Err(e) => {
            println!("{}", style::error(&format!("Error: {e}")));
            Err(anyhow::anyhow!("Config validation failed: {e}"))
        }
    }
}

fn run_validate_global() -> Result<()> {
    match Config::validate_global() {
        Ok(config) => {
            println!("{}\n", style::success("Valid"));
            print_config_summary(&config);
            Ok(())
        }
        Err(ConfigError::NotFound { .. }) => {
            let path_desc = global_config_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| {
                    format!("<config_dir>/{GLOBAL_CONFIG_DIR}/{GLOBAL_CONFIG_FILENAME}")
                });
            println!("No global config found at {path_desc}");
            Ok(())
        }
        Err(e) => {
            println!("{}", style::error(&format!("Error: {e}")));
            Err(anyhow::anyhow!("Config validation failed: {e}"))
        }
    }
}

fn run_show(path: Option<String>) -> Result<()> {
    let dir = path.unwrap_or_else(|| ".".to_string());

    // Show config source info
    match global_config_path() {
        Some(p) if p.exists() => println!("Global config: {}", p.display()),
        Some(p) => println!("Global config: {} (not found)", p.display()),
        None => println!("Global config: not available on this platform"),
    }

    let local_path = Path::new(&dir).join(CONFIG_FILENAME);
    if local_path.exists() {
        println!("Local config:  {}", local_path.display());
    } else {
        println!("Local config:  {} (not found)", local_path.display());
    }
    println!();

    let config = Config::load(&dir).unwrap_or_default();
    println!("{}", serde_json::to_string_pretty(&config)?);
    Ok(())
}

fn print_config_summary(config: &Config) {
    println!("Resolved configuration:");
    println!(
        "  output.max_tokens          = {}",
        config
            .output
            .max_tokens
            .map_or("null (unlimited)".to_string(), |v| v.to_string())
    );
    println!(
        "  output.truncate_signatures = {}",
        config.output.truncate_signatures
    );
    println!(
        "  output.truncate_doc_comments = {}",
        config.output.truncate_doc_comments
    );
    println!(
        "  search.max_results         = {}",
        config.search.max_results
    );
    println!(
        "  indexer.ignore              = {:?}",
        config.indexer.ignore
    );
    println!(
        "  indexer.max_file_size       = {}",
        config.indexer.max_file_size
    );
}
