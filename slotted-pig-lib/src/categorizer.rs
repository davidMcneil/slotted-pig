use std::{fs::File, io::BufReader, path::Path};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use derive_more::{Display, From, Into};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::transaction::Transaction;

#[derive(Error, Debug, Display)]
pub enum Error {
    /// io: {0}
    Io(#[from] std::io::Error),
    /// serde_yaml: {0}
    SerdeYaml(#[from] serde_yaml::Error),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryMatcher {
    pub category: String,
    pub min: Option<BigDecimal>,
    pub max: Option<BigDecimal>,
    #[serde(with = "serde_regex", default)]
    pub account: Option<Regex>,
    #[serde(with = "serde_regex", default)]
    pub description: Option<Regex>,
    pub begin: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

impl CategoryMatcher {
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
            .begin
            .as_ref()
            .map(|a| a <= &transaction.time)
            .unwrap_or(true);
        min && max && account && description && begin && end
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    pub category: String,
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Categorizer {
    category_matchers: Vec<CategoryMatcher>,
    category_hierarchy: Vec<Category>,
}

impl Categorizer {
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Ok(serde_yaml::from_reader(reader)?)
    }

    pub fn categorize(&self, transactions: &[Transaction]) -> CategorizedHierarchy {
        let categorized_transactions = transactions
            .iter()
            .flat_map(|transaction| {
                if let Some(matcher) = self
                    .category_matchers
                    .iter()
                    .find(|f| f.matches(transaction))
                {
                    Some((matcher.category.clone(), transaction.clone()))
                } else {
                    eprintln!("No filter matched {:?}", transaction);
                    None
                }
            })
            .collect::<Vec<_>>();

        self.category_hierarchy
            .iter()
            .map(|category| category.categorize(&categorized_transactions))
            .collect::<Vec<_>>()
            .into()
    }
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Categorized {
    pub category: String,
    pub total: BigDecimal,
    pub subcategories: Vec<Categorized>,
    pub transactions: Vec<Transaction>,
}

#[derive(Debug, Into, From, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategorizedHierarchy {
    pub categorized: Vec<Categorized>,
}
