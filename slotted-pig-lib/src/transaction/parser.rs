use serde::{Deserialize, Serialize};

/// Configuration for parsing transactions from csv files
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionParserCsvConfig {
    pub amount_fields: Vec<String>,
    pub account_fields: Vec<String>,
    pub description_fields: Vec<String>,
    pub time_fields: Vec<String>,
}

/// Configuration for parsing transactions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TransactionParserConfig {
    pub csv: TransactionParserCsvConfig
}
