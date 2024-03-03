use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use sloggers::{
    terminal::TerminalLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};
use slotted_pig_lib::{categorizer::Categorizer, transaction::TransactionParser};

/// The simple finance tracker
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to config file for reading transactions from files
    #[arg(long)]
    transaction_parser_path: PathBuf,
    /// File glob pattern of transaction files to parse
    #[arg(long)]
    transaction_path_pattern: String,
    /// Path to config file categorizing transactions
    #[arg(long)]
    categorizer_path: PathBuf,
    /// Log level
    #[arg(long)]
    log_level: Option<Severity>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let _logger = TerminalLoggerBuilder::new()
        .level(args.log_level.unwrap_or_default())
        .source_location(SourceLocation::None)
        .build()?;

    let categorizer = Categorizer::from_yaml_file(args.categorizer_path)
        .context("Failed to parse categorizer")?;
    let transaction_parser = TransactionParser::from_yaml_file(args.transaction_parser_path)
        .context("Failed to parse transaction parser")?;
    let transaction_files = glob::glob(&args.transaction_path_pattern)?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to find transaction files")?;
    let transaction_files = transaction_files
        .iter()
        .filter_map(|f| f.is_file().then(|| f.as_path()));
    let transactions = transaction_parser
        .parse_csvs(transaction_files)
        .context("Failed to parse transaction files")?;

    let categorized = categorizer.categorize(&transactions);
    println!("{}", serde_yaml::to_string(&categorized)?);

    Ok(())
}
