use std::{fs::File, path::Path};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// io: {0}
    Io(#[from] std::io::Error),
    /// csv: {0}
    Csv(#[from] csv::Error),
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
    pub fn from_csv_file<P: AsRef<Path>>(path: P) -> Result<Vec<Transaction>, Error> {
        let file = File::open(path)?;
        let mut reader = csv::Reader::from_reader(file);
        Ok(reader.deserialize().collect::<Result<_, _>>()?)
    }
}
