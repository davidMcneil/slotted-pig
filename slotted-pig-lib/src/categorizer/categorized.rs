use bigdecimal::BigDecimal;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

use crate::transaction::Transaction;

/// Categorized transaction hierarchy
#[derive(Debug, Default, Into, From, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategorizedList {
    pub categorized: Vec<Categorized>,
}

/// Categorized transactions
///
/// TODO: Avoid copying the data
#[derive(Debug, Deserialize, Serialize)]
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
    /// Possible children, either a list of transactions or categories
    pub children: CategorizedChildren,
}

// Possible categorized children, either a list of transactions or subcategories
#[derive(Debug, Deserialize, From, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CategorizedChildren {
    /// Child transactions
    Transactions(Vec<Transaction>),
    /// Child categories
    Subcategories(Vec<Categorized>),
}
