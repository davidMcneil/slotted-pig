use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
    str::FromStr,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use csv::ReaderBuilder;
use dateparser;
use derive_more::Display;
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

/// Configuration for parsing transactions from csv files
#[serde_as]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionParserCsvConfig {
    /// Regex to check if a file should be parsed with this config
    #[serde_as(as = "FromInto<RegexSerde>")]
    pub filename_regex: Regex,
    /// A preset account value to use for all transactions parsed with this config
    ///
    /// This will override the `account_header` setting
    pub account: Option<String>,
    /// Possible headers to use for the amount column
    #[serde(default = "TransactionParserCsvConfig::default_amount_header")]
    pub amount_header: String,
    /// Possible headers to use for the account column
    #[serde(default = "TransactionParserCsvConfig::default_account_header")]
    pub account_header: String,
    /// Possible headers to use for the description column
    #[serde(default = "TransactionParserCsvConfig::default_description_header")]
    pub description_header: String,
    /// Possible headers to use for the time column
    #[serde(default = "TransactionParserCsvConfig::default_time_header")]
    pub time_header: String,
}

impl TransactionParserCsvConfig {
    pub fn parse_csv<R: Read>(&self, reader: R) -> Result<Vec<Transaction>, Error> {
        enum AccountNameOrIndex<'a> {
            Name(&'a str),
            Index(usize),
        }

        let mut transactions = Vec::new();
        let mut reader = ReaderBuilder::new().from_reader(reader);

        // Read the headers and if the file is empty return an empty list
        let headers = reader.headers()?;
        if headers.is_empty() {
            return Ok(transactions);
        }

        // Find indices of headers
        // TODO: find indices for files without header
        let mut amount_index = None;
        let mut account_index = None;
        let mut description_index = None;
        let mut time_index = None;
        for (idx, header) in headers.iter().enumerate() {
            if header == self.amount_header {
                amount_index = Some(idx);
            }
            if header == self.account_header {
                account_index = Some(idx);
            }
            if header == self.description_header {
                description_index = Some(idx);
            }
            if header == self.time_header {
                time_index = Some(idx);
            }
        }
        let amount_index =
            amount_index.ok_or_else(|| Error::MissingAmount(headers.as_slice().into()))?;
        let account_name_or_index = if let Some(account) = &self.account {
            AccountNameOrIndex::Name(account)
        } else {
            AccountNameOrIndex::Index(
                account_index.ok_or_else(|| Error::MissingAccount(headers.as_slice().into()))?,
            )
        };
        let description_index = description_index
            .ok_or_else(|| Error::MissingDescription(headers.as_slice().into()))?;
        let time_index = time_index.ok_or_else(|| Error::MissingTime(headers.as_slice().into()))?;

        // Convert each row to a `Transaction` and add it to the list of transactions
        for result in reader.records() {
            let record = result?;

            // Get the &str for each column
            let amount = record
                .get(amount_index)
                .ok_or_else(|| Error::MissingAmount(record.as_slice().into()))?;
            let account = match account_name_or_index {
                AccountNameOrIndex::Name(account) => account,
                AccountNameOrIndex::Index(index) => record
                    .get(index)
                    .ok_or_else(|| Error::MissingAccount(record.as_slice().into()))?,
            };
            let description = record
                .get(description_index)
                .ok_or_else(|| Error::MissingDescription(record.as_slice().into()))?;
            let time = record
                .get(time_index)
                .ok_or_else(|| Error::MissingTime(record.as_slice().into()))?;

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

    fn default_amount_header() -> String {
        String::from("amount")
    }

    fn default_account_header() -> String {
        String::from("account")
    }

    fn default_description_header() -> String {
        String::from("description")
    }

    fn default_time_header() -> String {
        String::from("time")
    }
}

impl Default for TransactionParserCsvConfig {
    fn default() -> Self {
        Self {
            filename_regex: Regex::new(".*").expect("failed to compile default regex"),
            account: None,
            amount_header: Self::default_amount_header(),
            account_header: Self::default_account_header(),
            description_header: Self::default_description_header(),
            time_header: Self::default_time_header(),
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
