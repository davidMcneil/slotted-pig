use std::cmp::Reverse;

use bigdecimal::BigDecimal;
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};
use strum::EnumString;

use crate::transaction::Transaction;

/// Categorized transaction hierarchy
#[derive(Clone, Debug, Default, Deserialize, Into, From, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CategorizedList {
    pub categorized: Vec<Categorized>,
}

impl CategorizedList {
    /// Sort categories by the sort type
    pub fn sort_subcategories(&mut self, sort: CategorySort) {
        Categorized::sort_categorized(&mut self.categorized, sort)
    }

    /// Sort transactions by the sort type
    pub fn sort_transactions(&mut self, sort: TransactionSort) {
        self.categorized
            .iter_mut()
            .for_each(|c| c.sort_transactions(sort));
    }
}

/// Categorized transactions
///
/// TODO: Avoid copying the data
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

impl Categorized {
    fn sort_subcategories(&mut self, sort: CategorySort) {
        let CategorizedChildren::Subcategories(categories) = &mut self.children else {
            return;
        };
        Self::sort_categorized(categories, sort);
    }

    fn sort_categorized(categorized: &mut [Categorized], sort: CategorySort) {
        match sort {
            CategorySort::TotalDescending => categorized.sort_by(|c1, c2| c2.total.cmp(&c1.total)),
            CategorySort::TotalAscending => categorized.sort_by(|c1, c2| c1.total.cmp(&c2.total)),
            CategorySort::AbsoluteTotalDescending => {
                categorized.sort_by(|c1, c2| c2.absolute_total.cmp(&c1.absolute_total))
            }
            CategorySort::AbsoluteTotalAscending => {
                categorized.sort_by(|c1, c2| c1.absolute_total.cmp(&c2.absolute_total))
            }
            CategorySort::NameDescending => {
                categorized.sort_by(|c1, c2| c2.category.cmp(&c1.category))
            }
            CategorySort::NameAscending => {
                categorized.sort_by(|c1, c2| c1.category.cmp(&c2.category))
            }
        }
        categorized
            .iter_mut()
            .for_each(|c| c.sort_subcategories(sort));
    }

    fn sort_transactions(&mut self, sort: TransactionSort) {
        match &mut self.children {
            CategorizedChildren::Transactions(transactions) => match sort {
                TransactionSort::TimeDescending => transactions.sort_by_key(|t| Reverse(t.time)),
                TransactionSort::TimeAscending => transactions.sort_by_key(|t| t.time),
                TransactionSort::AmountDescending => {
                    transactions.sort_by(|t1, t2| t2.amount.cmp(&t1.amount))
                }
                TransactionSort::AmountAscending => {
                    transactions.sort_by(|t1, t2| t1.amount.cmp(&t2.amount))
                }
                TransactionSort::AbsoluteAmountDescending => transactions
                    .sort_by(|t1: &Transaction, t2| t2.amount.abs().cmp(&t1.amount.abs())),
                TransactionSort::AbsoluteAmountAscending => {
                    transactions.sort_by(|t1, t2| t1.amount.abs().cmp(&t2.amount.abs()))
                }
            },
            CategorizedChildren::Subcategories(subcategories) => {
                subcategories
                    .iter_mut()
                    .for_each(|c| c.sort_transactions(sort));
            }
        }
    }
}

// Possible categorized children, either a list of transactions or subcategories
#[derive(Clone, Debug, Deserialize, From, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CategorizedChildren {
    /// Child transactions
    Transactions(Vec<Transaction>),
    /// Child categories
    Subcategories(Vec<Categorized>),
}

/// Sort possibilities for scategories
#[derive(Clone, Copy, Debug, Deserialize, EnumString, Serialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CategorySort {
    /// Sort by category total descending
    TotalDescending,
    /// Sort by category total ascending
    TotalAscending,
    /// Sort by category absolute total descending
    AbsoluteTotalDescending,
    /// Sort by category absolute total ascending
    AbsoluteTotalAscending,
    /// Sort by category name descending
    NameDescending,
    /// Sort by category name ascending
    NameAscending,
}

/// Sort possibilities for transactions
#[derive(Clone, Copy, Debug, Deserialize, EnumString, Serialize)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum TransactionSort {
    /// Sort by transaction time descending
    TimeDescending,
    /// Sort by transaction time ascending
    TimeAscending,
    /// Sort by transaction amount descending
    AmountDescending,
    /// Sort by transaction amount ascending
    AmountAscending,
    /// Sort by transaction absolute amount descending
    AbsoluteAmountDescending,
    /// Sort by transaction absolute amount ascending
    AbsoluteAmountAscending,
}
