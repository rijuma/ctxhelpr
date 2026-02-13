use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::Path;

use crate::config::{CONFIG_FILENAME, Config, ConfigError};

#[derive(Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommand,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Create a .ctxhelpr.json template in the current directory
    Init,
    /// Validate an existing .ctxhelpr.json file
    Validate {
        /// Directory containing .ctxhelpr.json (defaults to current directory)
        #[arg(long)]
        path: Option<String>,
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
        ConfigCommand::Init => run_init(),
        ConfigCommand::Validate { path } => run_validate(path),
        ConfigCommand::Show { path } => run_show(path),
    }
}

fn run_init() -> Result<()> {
    let config_path = Path::new(CONFIG_FILENAME);
    if config_path.exists() {
        println!(
            "{} already exists in the current directory.",
            CONFIG_FILENAME
        );
        return Ok(());
    }

    let template = serde_json::to_string_pretty(&Config::default())?;
    std::fs::write(config_path, format!("{template}\n"))?;
    println!("Created {}", config_path.display());
    Ok(())
}

fn run_validate(path: Option<String>) -> Result<()> {
    let dir = path.unwrap_or_else(|| ".".to_string());
    match Config::validate(&dir) {
        Ok(config) => {
            println!("Valid\n");
            print_config_summary(&config);
            Ok(())
        }
        Err(ConfigError::NotFound) => {
            println!("No {} found in {}", CONFIG_FILENAME, dir);
            Ok(())
        }
        Err(e) => {
            println!("Error: {e}");
            std::process::exit(1);
        }
    }
}

fn run_show(path: Option<String>) -> Result<()> {
    let dir = path.unwrap_or_else(|| ".".to_string());
    let config_path = Path::new(&dir).join(CONFIG_FILENAME);

    if !config_path.exists() {
        println!("No {} found, using defaults.\n", CONFIG_FILENAME);
    }

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
