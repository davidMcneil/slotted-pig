use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
    str::FromStr,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder, StringRecord};
use dateparser;
use derive_more::{Display, From};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, FromInto};
use thiserror::Error;

use crate::util::RegexSerde;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// bigdecimal: {0}
    BigDecimal(#[from] bigdecimal::ParseBigDecimalError),
    /// csv: {0}
    Csv(#[from] csv::Error),
    /// dateparser: {0}
    Dateparser(#[from] anyhow::Error),
    /// io: {0}
    Io(#[from] std::io::Error),
    /// Invalid path to file: {0}
    InvalidPathToFile(String),
    /// Missing amount: {0}
    MissingAmount(String),
    /// Missing account: {0}
    MissingAccount(String),
    /// Missing description: {0}
    MissingDescription(String),
    /// Missing time: {0}
    MissingTime(String),
    /// No matching csv parser config: {0}
    NoMatchingCsvConfig(String),
}

/// Transaction
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    /// Amount of the transaction
    pub amount: BigDecimal,
    /// Account the transaction applied to
    pub account: String,
    /// Description of the transaction
    pub description: String,
    /// Time of the transaction
    pub time: DateTime<Utc>,
}

impl Transaction {
    /// Create a new list of transactions from a csv file
    pub fn from_csv_file<P: AsRef<Path>>(path: P) -> Result<Vec<Self>, Error> {
        Self::from_reader(File::open(path)?)
    }

    /// Create a new list of transactions from a csv buffer
    pub fn from_csv_buffer<B: AsRef<[u8]>>(buffer: B) -> Result<Vec<Self>, Error> {
        Self::from_reader(Cursor::new(buffer))
    }

    fn from_reader<R: Read>(reader: R) -> Result<Vec<Self>, Error> {
        TransactionParserCsvConfig::default().parse_csv(reader)
    }
}

/// Determine if a columns values should be decided by a header, index, or constant
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnDeterminer {
    /// Column is a constant value
    Constant(String),
    /// Column is determined by a header
    Header(String),
    /// Column is determined by an index
    Index(usize),
}

impl ColumnDeterminer {
    fn constant_or_index(&self, headers: &StringRecord) -> Result<ConstantOrIndex, String> {
        match self {
            Self::Constant(constant) => Ok(constant.as_str().into()),
            Self::Header(header) => headers
                .iter()
                .position(|h| h == header)
                .map(Into::into)
                .ok_or_else(|| headers.as_slice().into()),
            Self::Index(index) => Ok((*index).into()),
        }
    }
}

/// A type to retrieve a columns value from a csv row
#[derive(From)]
enum ConstantOrIndex<'a> {
    Constant(&'a str),
    Index(usize),
}

impl<'a> ConstantOrIndex<'a> {
    fn value(&self, row: &'a StringRecord) -> Result<&'a str, String> {
        match *self {
            Self::Constant(constant) => Ok(constant),
            Self::Index(index) => row.get(index).ok_or_else(|| row.as_slice().into()),
        }
    }
}

/// Configuration for parsing transactions from csv files
#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionParserCsvConfig {
    /// Regex to check if a file should be parsed with this config
    #[serde_as(as = "FromInto<RegexSerde>")]
    pub filename_regex: Regex,
    /// Does this file have a header?
    #[serde(default = "TransactionParserCsvConfig::default_has_header")]
    pub has_header: bool,
    /// Possible headers to use for the amount column
    #[serde(default = "TransactionParserCsvConfig::default_amount_column")]
    pub amount_column: ColumnDeterminer,
    /// Possible headers to use for the account column
    #[serde(default = "TransactionParserCsvConfig::default_account_column")]
    pub account_column: ColumnDeterminer,
    /// Possible headers to use for the description column
    #[serde(default = "TransactionParserCsvConfig::default_description_column")]
    pub description_column: ColumnDeterminer,
    /// Possible headers to use for the time column
    #[serde(default = "TransactionParserCsvConfig::default_time_column")]
    pub time_column: ColumnDeterminer,
}

impl TransactionParserCsvConfig {
    pub fn parse_csv<R: Read>(&self, reader: R) -> Result<Vec<Transaction>, Error> {
        let mut transactions = Vec::new();
        let mut reader = ReaderBuilder::new()
            .has_headers(self.has_header)
            .from_reader(reader);

        // Read the headers and if the file is empty return an empty list
        let headers = reader.headers()?;
        if headers.is_empty() {
            return Ok(transactions);
        }

        // Find indexes of headers
        // TODO: find indices for files without header
        let amount_constant_or_index = self
            .amount_column
            .constant_or_index(headers)
            .map_err(Error::MissingAmount)?;
        let account_constant_or_index = self
            .account_column
            .constant_or_index(headers)
            .map_err(Error::MissingAccount)?;
        let description_constant_or_index = self
            .description_column
            .constant_or_index(headers)
            .map_err(Error::MissingDescription)?;
        let time_constant_or_index = self
            .time_column
            .constant_or_index(headers)
            .map_err(Error::MissingTime)?;

        // Convert each row to a `Transaction` and add it to the list of transactions
        for result in reader.records() {
            let record = &result?;

            // Get the &str for each column
            let amount = amount_constant_or_index
                .value(record)
                .map_err(Error::MissingAmount)?;
            let account = account_constant_or_index
                .value(record)
                .map_err(Error::MissingAccount)?;
            let description = description_constant_or_index
                .value(record)
                .map_err(Error::MissingDescription)?;
            let time = time_constant_or_index
                .value(record)
                .map_err(Error::MissingTime)?;

            // Special parsing or conversion for each column
            let amount = BigDecimal::from_str(amount)?;
            let account = account.to_string();
            let description = description.to_string();
            let time = dateparser::parse(time)?;

            let transaction = Transaction {
                amount,
                account,
                description,
                time,
            };
            transactions.push(transaction);
        }

        Ok(transactions)
    }

    fn default_has_header() -> bool {
        true
    }

    fn default_amount_column() -> ColumnDeterminer {
        ColumnDeterminer::Header(String::from("amount"))
    }

    fn default_account_column() -> ColumnDeterminer {
        ColumnDeterminer::Header(String::from("account"))
    }

    fn default_description_column() -> ColumnDeterminer {
        ColumnDeterminer::Header(String::from("description"))
    }

    fn default_time_column() -> ColumnDeterminer {
        ColumnDeterminer::Header(String::from("time"))
    }
}

impl Default for TransactionParserCsvConfig {
    fn default() -> Self {
        Self {
            filename_regex: Regex::new(".*").expect("failed to compile default regex"),
            has_header: Self::default_has_header(),
            amount_column: Self::default_amount_column(),
            account_column: Self::default_account_column(),
            description_column: Self::default_description_column(),
            time_column: Self::default_time_column(),
        }
    }
}

/// Configuration for parsing transactions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionParserConfig {
    #[serde(default)]
    pub csvs: Vec<TransactionParserCsvConfig>,
}

impl TransactionParserConfig {
    pub fn parse_csvs(&self, paths: &[&Path]) -> Result<Vec<Transaction>, Error> {
        let mut transactions = Vec::new();

        for path in paths {
            let filename = path
                .file_name()
                .and_then(|f| f.to_str())
                .ok_or_else(|| Error::InvalidPathToFile(path.display().to_string()))?;

            // Find the csv parsing config that matches this filename
            let csv_config = self
                .csvs
                .iter()
                .find(|csv| csv.filename_regex.is_match(filename))
                .ok_or_else(|| Error::NoMatchingCsvConfig(path.display().to_string()))?;

            // Parse the file
            let file = File::open(path)?;
            let new_transactions = csv_config.parse_csv(file)?;

            // TODO: check for duplicate transactions
            transactions.extend(new_transactions);
        }

        Ok(transactions)
    }
}
