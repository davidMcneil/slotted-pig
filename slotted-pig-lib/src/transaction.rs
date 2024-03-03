use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
    path::{Path, PathBuf},
    str::FromStr,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use csv::{ReaderBuilder, StringRecord};
use dateparser;
use derive_more::From;
use displaydoc::Display;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, FromInto};
use thiserror::Error;

use crate::util::RegexSerde;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// bigdecimal
    BigDecimal(#[from] bigdecimal::ParseBigDecimalError),
    /// csv
    Csv(#[from] csv::Error),
    /// dateparser
    Dateparser(#[from] anyhow::Error),
    /// io
    Io(#[from] std::io::Error),
    /// invalid path to file: {0}
    InvalidPathToFile(PathBuf),
    /// missing amount: {0}
    MissingAmount(String),
    /// missing account: {0}
    MissingAccount(String),
    /// missing description: {0}
    MissingDescription(String),
    /// missing time: {0}
    MissingTime(String),
    /// no matching csv parser config: {0}
    NoMatchingCsvConfig(PathBuf),
    /// serde_yaml: {0}
    SerdeYaml(#[from] serde_yaml::Error),
    // TODO: this is kinda a hack and should be its own error type
    /// failed to parse: {0}
    ParseFailed(PathBuf, #[source] Box<Self>),
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
        TransactionParserCsv::default().parse_csv(reader)
    }
}

/// Configuration for parsing transactions
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionParser {
    #[serde(default)]
    pub csv: Vec<TransactionParserCsv>,
}

impl TransactionParser {
    /// Create a new categorizer from a yaml file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        Self::from_reader(File::open(path)?)
    }

    /// Create a new categorizer from a yaml buffer
    pub fn from_yaml_buffer<B: AsRef<[u8]>>(buffer: B) -> Result<Self, Error> {
        Self::from_reader(Cursor::new(buffer))
    }

    fn from_reader<R: Read>(reader: R) -> Result<Self, Error> {
        let reader = BufReader::new(reader);
        Ok(serde_yaml::from_reader(reader)?)
    }

    /// Parse transactions from CSV files
    pub fn parse_csvs<'a>(
        &self,
        paths: impl Iterator<Item = &'a Path>,
    ) -> Result<Vec<Transaction>, Error> {
        let mut transactions = Vec::new();
        for path in paths {
            let new_transactions = self.parse_csv(path)?;
            // TODO: check for duplicate transactions
            transactions.extend(new_transactions);
        }
        Ok(transactions)
    }

    /// Parse transactions from a CSV files
    pub fn parse_csv(&self, path: &Path) -> Result<Vec<Transaction>, Error> {
        self.parse_csv_impl(path)
            .map_err(|e| Error::ParseFailed(path.into(), e.into()))
    }

    fn parse_csv_impl(&self, path: &Path) -> Result<Vec<Transaction>, Error> {
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| Error::InvalidPathToFile(path.into()))?;

        // Find the csv parsing config that matches this filename
        let csv_config = self
            .csv
            .iter()
            .find(|csv| csv.filename_regex.is_match(filename))
            .ok_or_else(|| Error::NoMatchingCsvConfig(path.into()))?;

        // Parse the file
        let file = File::open(path)?;
        csv_config.parse_csv(file)
    }
}

/// Configuration for parsing transactions from csv files
#[serde_as]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionParserCsv {
    /// Regex to check if a file should be parsed with this config
    #[serde_as(as = "FromInto<RegexSerde>")]
    pub filename_regex: Regex,
    /// Does this file have a header?
    #[serde(default = "TransactionParserCsv::default_has_header")]
    pub has_header: bool,
    /// Possible headers to use for the amount column
    #[serde(default = "TransactionParserCsv::default_amount_column")]
    pub amount_column: ColumnDeterminer,
    /// Possible headers to use for the account column
    #[serde(default = "TransactionParserCsv::default_account_column")]
    pub account_column: ColumnDeterminer,
    /// Possible headers to use for the description column
    #[serde(default = "TransactionParserCsv::default_description_column")]
    pub description_column: ColumnDeterminer,
    /// Possible headers to use for the time column
    #[serde(default = "TransactionParserCsv::default_time_column")]
    pub time_column: ColumnDeterminer,
}

impl TransactionParserCsv {
    fn parse_csv<R: Read>(&self, reader: R) -> Result<Vec<Transaction>, Error> {
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

impl Default for TransactionParserCsv {
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

/// Determine if a columns values should be decided by a header, index, or constant
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
