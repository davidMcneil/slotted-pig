use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use derive_more::{From, Into};
use displaydoc::Display;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_with::{formats::PreferOne, serde_as, FromInto, OneOrMany};
use thiserror::Error;

use crate::{
    transaction::Transaction,
    util::{self, RegexSerde},
};

#[derive(Error, Debug, Display)]
pub enum Error {
    /// io
    Io(#[from] std::io::Error),
    /// serde_yaml
    SerdeYaml(#[from] serde_yaml::Error),
    /// duplicate categories in transaction matchers: {0:?}
    DuplicateCategoriesInTransactionMatchers(HashSet<String>),
    /// duplicate categories in category hierarchy: {0:?}
    DuplicateCategoriesInCategoryHierarchy(HashSet<String>),
    /// missing categories in category hierarchy: {0:?}
    MissingCategoriesInCategoryHierarchy(HashSet<String>),
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
    pub account: Option<String>,
    /// Regex to match against the description of the transaction
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
        let mut count = 0;
        let mut total = BigDecimal::default();
        let mut absolute_total = BigDecimal::default();

        let subcategories = self
            .subcategories
            .iter()
            .map(|subcategory| {
                let categorized = subcategory.categorize(transactions);
                count += categorized.count;
                total += categorized.total.clone();
                absolute_total += categorized.absolute_total.clone();
                categorized
            })
            .collect();

        let transactions = transactions
            .iter()
            .filter(|(c, _)| c == &self.category)
            .map(|(_, transaction)| {
                count += 1;
                total += transaction.amount.clone();
                absolute_total += transaction.amount.abs();
                transaction.clone()
            })
            .collect();

        Categorized {
            category: self.category.clone(),
            count,
            total,
            absolute_total,
            subcategories,
            transactions,
        }
    }

    fn categories(&self) -> Vec<&str> {
        let mut categories = vec![self.category.as_str()];
        for subcategory in &self.subcategories {
            categories.extend(subcategory.categories())
        }
        categories
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

    /// Create a new categorizer from a reader
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, Error> {
        let reader = BufReader::new(reader);
        let categorizer = serde_yaml::from_reader::<_, Self>(reader)?;
        categorizer.validate_categories()?;
        Ok(categorizer)
    }

    /// Validate categories:
    /// * no duplicate categories in transaction matchers or category hierarchy
    /// * all transaction matcher categories exist in the category hierarchy
    pub fn validate_categories(&self) -> Result<(), Error> {
        let (matchers_unique, matchers_duplicates) =
            util::vec_to_hashsets(self.matchers.iter().map(|m| m.category.as_str()));
        if !matchers_duplicates.is_empty() {
            return Err(Error::DuplicateCategoriesInTransactionMatchers(
                matchers_duplicates.into_iter().map(|c| c.into()).collect(),
            ));
        }
        let (hierarchy_unique, hierarchy_duplicates) =
            util::vec_to_hashsets(self.hierarchy.iter().flat_map(|c| c.categories()));
        if !hierarchy_duplicates.is_empty() {
            return Err(Error::DuplicateCategoriesInCategoryHierarchy(
                hierarchy_duplicates.into_iter().map(|c| c.into()).collect(),
            ));
        }
        let mut hierarchy_missing = matchers_unique.difference(&hierarchy_unique).peekable();
        if hierarchy_missing.peek().is_some() {
            return Err(Error::MissingCategoriesInCategoryHierarchy(
                hierarchy_missing.map(|c| (*c).into()).collect(),
            ));
        }
        Ok(())
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
/// TODO: subcategories and transactions could be made into an enum as they are mutually exclusive
#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Categorized {
    /// Category name
    pub category: String,
    /// Count of transactions in this category (ie sum of all subcategory counts)
    pub count: u64,
    /// Total amount in this category (ie sum of all subcategory totals)
    pub total: BigDecimal,
    /// Total absolute amount in this category (ie sum of all subcategory absolute totals)
    pub absolute_total: BigDecimal,
    /// Subcategories to consider as part of this category
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcategories: Vec<Categorized>,
    /// Transactions which matched this category (only leaf categories have transactions)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub transactions: Vec<Transaction>,
}

/// Categorized transaction hierarchy
#[derive(Debug, Default, Into, From, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategorizedHierarchy {
    pub categorized: Vec<Categorized>,
}
