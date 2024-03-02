use std::{
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use derive_more::{Display, From, Into};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, FromInto};
use thiserror::Error;

use crate::{transaction::Transaction, util::RegexSerde};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// io: {0}
    Io(#[from] std::io::Error),
    /// serde_yaml: {0}
    SerdeYaml(#[from] serde_yaml::Error),
}

/// Rules to determine if a transaction matches a category
#[serde_as]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionMatcher {
    /// Category name for this matcher
    pub category: String,
    /// Minimum amount of the transaction inclusive
    pub min: Option<BigDecimal>,
    /// Maximum amount of the transaction inclusive
    pub max: Option<BigDecimal>,
    /// Regex to match against the account of the transaction
    #[serde_as(as = "Option<FromInto<RegexSerde>>")]
    pub account: Option<Regex>,
    /// Regex to match against the description of the transaction
    #[serde_as(as = "Option<FromInto<RegexSerde>>")]
    pub description: Option<Regex>,
    /// Time inclusive after which the transaction must have occurred
    pub begin: Option<DateTime<Utc>>,
    /// Time inclusive before which the transaction must have occurred
    pub end: Option<DateTime<Utc>>,
}

impl TransactionMatcher {
    /// Check if a transaction is a match
    fn matches(&self, transaction: &Transaction) -> bool {
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
            .map(|r| r.is_match(&transaction.account))
            .unwrap_or(true);
        let description = self
            .description
            .as_ref()
            .map(|r| r.is_match(&transaction.description))
            .unwrap_or(true);
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

/// Hierarchy of categories with arbitrary depth
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Category name
    pub category: String,
    /// Subcategories to consider as part of this category
    #[serde(default)]
    pub subcategories: Vec<Category>,
}

impl Category {
    fn categorize(&self, transactions: &[(String, Transaction)]) -> Categorized {
        let mut total = BigDecimal::default();

        let subcategories = self
            .subcategories
            .iter()
            .map(|subcategory| {
                let categorized = subcategory.categorize(transactions);
                total += categorized.total.clone();
                categorized
            })
            .collect();

        let transactions = transactions
            .iter()
            .filter(|(c, _)| c == &self.category)
            .map(|(_, transaction)| {
                total += transaction.amount.clone();
                transaction.clone()
            })
            .collect();

        Categorized {
            category: self.category.clone(),
            total,
            subcategories,
            transactions,
        }
    }
}

/// Transaction categorizer
///
/// Matchers are use to assign transactions to leaf categories and then a category hierarchy is
/// constructed
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Categorizer {
    /// Transaction matcher rules
    pub matchers: Vec<TransactionMatcher>,
    /// Category hierarchy
    pub hierarchy: Vec<Category>,
}

impl Categorizer {
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

    /// Categorize transactions returning a new category hierarchy
    pub fn categorize(&self, transactions: &[Transaction]) -> CategorizedHierarchy {
        let categorized_transactions = transactions
            .iter()
            .flat_map(|transaction| {
                if let Some(matcher) = self.matchers.iter().find(|f| f.matches(transaction)) {
                    Some((matcher.category.clone(), transaction.clone()))
                } else {
                    eprintln!("No filter matched {:?}", transaction);
                    None
                }
            })
            .collect::<Vec<_>>();

        self.hierarchy
            .iter()
            .map(|category| category.categorize(&categorized_transactions))
            .collect::<Vec<_>>()
            .into()
    }
}

/// Categorized transactions
///
/// TODO: Avoid copying the data
#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Categorized {
    /// Category name
    pub category: String,
    /// Total amount in this category (ie sum of all subcategory amounts)
    pub total: BigDecimal,
    /// Subcategories to consider as part of this category
    pub subcategories: Vec<Categorized>,
    /// Transactions which matched this category (only leaf categories have transactions)
    pub transactions: Vec<Transaction>,
}

/// Categorized transaction hierarchy
#[derive(Debug, Default, Into, From, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategorizedHierarchy {
    pub categorized: Vec<Categorized>,
}
