use std::{
    collections::HashSet,
    fs::File,
    io::{BufReader, Cursor, Read},
    path::Path,
};

use bigdecimal::BigDecimal;
use displaydoc::Display;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::transaction::Transaction;

pub use categorized::*;
pub use transaction_matcher::*;

mod categorized;
mod transaction_matcher;

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

/// Transaction categorizer
///
/// Categories are constructed into a hierarchy. Matchers are use to assign transactions to leaf
/// categories.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Categorizer {
    /// Filters to apply to transactions before doing any categorization
    pub transaction_filters: Option<Vec<TransactionMatcher>>,
    /// Category hierarchy
    pub categories: Vec<Category>,
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
        Ok(serde_yaml::from_reader::<_, Self>(reader)?)
    }

    /// Categorize transactions returning a new category hierarchy
    pub fn categorize<'a>(
        &self,
        transactions: &'a [Transaction],
    ) -> (CategorizedList, Vec<&'a Transaction>) {
        let transactions = transactions
            .iter()
            .filter(|t| {
                self.transaction_filters
                    .as_ref()
                    .map_or(true, |filters| filters.iter().any(|f| f.matches(t)))
            })
            .collect::<Vec<_>>();
        let mut categorized_transactions = HashSet::new();
        let categorized = self
            .categories
            .iter()
            .map(|category| category.categorize(&transactions, &mut categorized_transactions))
            .collect::<Vec<_>>()
            .into();
        let uncategorized = transactions
            .iter()
            .enumerate()
            .filter(|(i, _)| !categorized_transactions.contains(i))
            .map(|(_, t)| *t)
            .collect();
        (categorized, uncategorized)
    }
}

/// Hierarchy of categories with arbitrary depth
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Category name
    pub category: String,
    /// Category children
    pub children: CategoryChildren,
}

impl Category {
    fn categorize(
        &self,
        transactions: &[&Transaction],
        categorized: &mut HashSet<usize>,
    ) -> Categorized {
        let mut count = 0;
        let mut total = BigDecimal::default();
        let mut absolute_total = BigDecimal::default();

        let children = match &self.children {
            CategoryChildren::TransactionMatchers(matchers) => transactions
                .iter()
                .enumerate()
                .filter_map(|(i, t)| {
                    if !categorized.contains(&i) && matchers.iter().any(|m| m.matches(t)) {
                        // Mutating operations. This would be better written as `filter` and
                        // `inspect` but cant due to `categorized` being borrowed twice
                        count += 1;
                        total += t.amount.clone();
                        absolute_total += t.amount.abs();
                        categorized.insert(i);
                        Some(*t)
                    } else {
                        None
                    }
                })
                .cloned()
                .collect::<Vec<_>>()
                .into(),
            CategoryChildren::Subcategories(subcategories) => subcategories
                .iter()
                .map(|subcategory| {
                    let categorized = subcategory.categorize(transactions, categorized);
                    count += categorized.count;
                    total += categorized.total.clone();
                    absolute_total += categorized.absolute_total.clone();
                    categorized
                })
                .collect::<Vec<_>>()
                .into(),
        };

        Categorized {
            category: self.category.clone(),
            count,
            total,
            absolute_total,
            children,
        }
    }
}

// Possible category children, either a list of transaction matchers or subcategories
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryChildren {
    /// Transaction matchers
    TransactionMatchers(Vec<TransactionMatcher>),
    /// Subcategories
    Subcategories(Vec<Category>),
}
