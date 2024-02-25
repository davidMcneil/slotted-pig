use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

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
