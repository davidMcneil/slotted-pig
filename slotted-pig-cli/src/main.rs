use std::{io, path::PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use csv::Writer;
use sloggers::{
    terminal::TerminalLoggerBuilder,
    types::{Severity, SourceLocation},
    Build,
};
use slotted_pig_lib::{
    categorizer::{Categorizer, CategorySort, TransactionSort},
    transaction::{Transaction, TransactionParser},
};

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
    // Subcommands
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
#[command()]
enum Command {
    /// Output the categorized yaml
    #[command()]
    Categorize(Categorize),
    /// Output the transactions csv
    #[command()]
    Transactions,
}

#[derive(Debug, Parser)]
struct Categorize {
    /// How to sort the categories
    #[arg(long)]
    category_sort: Option<CategorySort>,
    /// How to sort the transactions
    #[arg(long)]
    transaction_sort: Option<TransactionSort>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let _logger = TerminalLoggerBuilder::new()
        .level(args.log_level.unwrap_or_default())
        .source_location(SourceLocation::None)
        .build()?;

    let categorizer = Categorizer::from_yaml_file(args.categorizer_path)
        .context("failed to parse categorizer")?;
    let transaction_parser = TransactionParser::from_yaml_file(args.transaction_parser_path)
        .context("failed to parse transaction parser")?;
    let transaction_files = glob::glob(&args.transaction_path_pattern)?
        .collect::<Result<Vec<_>, _>>()
        .context("failed to find transaction files")?;
    let transaction_files = transaction_files
        .iter()
        .filter(|f| f.is_file())
        .map(|f| f.as_path());
    let transactions = transaction_parser
        .parse_csvs(transaction_files)
        .context("failed to parse transaction files")?;

    match args.command {
        Command::Categorize(categorize) => {
            let (mut categorized, uncategorized) = categorizer.categorize(&transactions);
            write_transactions(&uncategorized, io::stderr())?;
            if let Some(sort) = categorize.category_sort {
                categorized.sort_subcategorized(sort);
            }
            if let Some(sort) = categorize.transaction_sort {
                categorized.sort_transactions(sort);
            }
            println!("{}", serde_yaml::to_string(&categorized)?);
        }
        Command::Transactions => {
            write_transactions(&transactions.iter().collect::<Vec<_>>(), io::stdout())?
        }
    }
    Ok(())
}

fn write_transactions<W: io::Write>(transactions: &[&Transaction], writer: W) -> Result<()> {
    let mut writer = Writer::from_writer(writer);
    for row in transactions {
        writer.serialize(row)?;
    }
    writer.flush()?;
    Ok(())
}
