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
use serde::{Deserialize, Serialize};
use thiserror::Error;

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
    /// Missing amount: {0:?}
    MissingAmount(String),
    /// Missing account: {0:?}
    MissingAccount(String),
    /// Missing description: {0:?}
    MissingDescription(String),
    /// Missing time: {0:?}
    MissingTime(String),
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
        let mut reader = csv::Reader::from_reader(reader);
        Ok(reader.deserialize().collect::<Result<_, _>>()?)
    }
}

/// Configuration for parsing transactions from csv files
#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct TransactionParserCsvConfig {
    #[serde(default)]
    pub amount_header: Vec<String>,
    #[serde(default)]
    pub account_header: Vec<String>,
    #[serde(default)]
    pub description_header: Vec<String>,
    #[serde(default)]
    pub time_header: Vec<String>,
}

/// Configuration for parsing transactions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionParserConfig {
    #[serde(default)]
    pub csv: TransactionParserCsvConfig,
}

impl TransactionParserConfig {
    fn parse_csv<R: Read>(&self, reader: R) -> Result<Vec<Transaction>, Error> {
        let mut transactions = Vec::new();
        let mut reader = ReaderBuilder::new().from_reader(reader);

        // Find indices of headers
        let headers = reader.headers()?;
        let mut amount_index = None;
        let mut account_index = None;
        let mut description_index = None;
        let mut time_index = None;
        for (idx, header) in headers.iter().enumerate() {
            if self.csv.amount_header.iter().any(|h| h == header) {
                amount_index = Some(idx);
            }
            if self.csv.account_header.iter().any(|h| h == header) {
                account_index = Some(idx);
            }
            if self.csv.description_header.iter().any(|h| h == header) {
                description_index = Some(idx);
            }
            if self.csv.time_header.iter().any(|h| h == header) {
                time_index = Some(idx);
            }
        }
        let amount_idx =
            amount_index.ok_or_else(|| Error::MissingAmount(headers.as_slice().into()))?;
        let account_idx =
            account_index.ok_or_else(|| Error::MissingAccount(headers.as_slice().into()))?;
        let description_idx = description_index
            .ok_or_else(|| Error::MissingDescription(headers.as_slice().into()))?;
        let time_idx = time_index.ok_or_else(|| Error::MissingTime(headers.as_slice().into()))?;

        for result in reader.records() {
            let record = result?;

            let amount = record
                .get(amount_idx)
                .ok_or_else(|| Error::MissingAmount(record.as_slice().into()))?;
            let account = record
                .get(account_idx)
                .ok_or_else(|| Error::MissingAccount(record.as_slice().into()))?
                .to_string();
            let description = record
                .get(description_idx)
                .ok_or_else(|| Error::MissingDescription(record.as_slice().into()))?
                .to_string();
            let time = record
                .get(time_idx)
                .ok_or_else(|| Error::MissingTime(record.as_slice().into()))?;

            let amount = BigDecimal::from_str(amount)?;
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
}
