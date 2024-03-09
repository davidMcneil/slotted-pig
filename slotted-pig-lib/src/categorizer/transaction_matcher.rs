use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{formats::PreferOne, serde_as, FromInto, OneOrMany};

use crate::{transaction::Transaction, util::RegexSerde};

/// Rules to determine if a transaction matches a category
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionMatcher {
    /// Minimum amount of the transaction inclusive
    pub min: Option<BigDecimal>,
    /// Maximum amount of the transaction inclusive
    pub max: Option<BigDecimal>,
    /// Match against account name of the transaction
    pub account: Option<String>,
    /// List of regex to match against the description of the transaction
    #[serde_as(as = "OneOrMany<FromInto<RegexSerde>, PreferOne>")]
    #[serde(default)]
    pub description: Vec<Regex>,
    /// Time inclusive after which the transaction must have occurred
    pub begin: Option<DateTime<Utc>>,
    /// Time inclusive before which the transaction must have occurred
    pub end: Option<DateTime<Utc>>,
}

impl TransactionMatcher {
    /// Check if a transaction is a match
    pub fn matches(&self, transaction: &Transaction) -> bool {
        let min = self
            .min
            .as_ref()
            .map(|a| a <= &transaction.amount)
            .unwrap_or(true);
        let max = self
            .max
            .as_ref()
            .map(|a| a >= &transaction.amount)
            .unwrap_or(true);
        let account = self
            .account
            .as_ref()
            .map(|a| a == &transaction.account)
            .unwrap_or(true);
        let description = self.description.is_empty()
            || self
                .description
                .iter()
                .any(|r| r.is_match(&transaction.description));
        let begin = self
            .begin
            .as_ref()
            .map(|a| a >= &transaction.time)
            .unwrap_or(true);
        let end = self
            .end
            .as_ref()
            .map(|a| a <= &transaction.time)
            .unwrap_or(true);
        min && max && account && description && begin && end
    }
}
