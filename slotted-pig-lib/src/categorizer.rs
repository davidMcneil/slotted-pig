use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use derive_more::{From, Into};
use regex::Regex;
use serde::Deserialize;

use crate::transaction::Transaction;

#[derive(Debug, Deserialize)]
pub(crate) struct CategoryMatcher {
    pub category: String,
    pub min_amount: Option<BigDecimal>,
    pub max_amount: Option<BigDecimal>,
    #[serde(with = "serde_regex")]
    pub account_regex: Option<Regex>,
    #[serde(with = "serde_regex")]
    pub description_regex: Option<Regex>,
    pub min_time: Option<DateTime<Utc>>,
    pub max_time: Option<DateTime<Utc>>,
}

impl CategoryMatcher {
    fn matches(&self, transaction: &Transaction) -> bool {
        let min_amount = self
            .min_amount
            .as_ref()
            .map(|a| a <= &transaction.amount)
            .unwrap_or(true);
        let max_amount = self
            .max_amount
            .as_ref()
            .map(|a| a >= &transaction.amount)
            .unwrap_or(true);
        let account_regex = self
            .account_regex
            .as_ref()
            .map(|r| r.is_match(&transaction.account))
            .unwrap_or(true);
        let description_regex = self
            .account_regex
            .as_ref()
            .map(|r| r.is_match(&transaction.description))
            .unwrap_or(true);
        let min_time = self
            .min_time
            .as_ref()
            .map(|a| a >= &transaction.time)
            .unwrap_or(true);
        let max_time = self
            .min_time
            .as_ref()
            .map(|a| a <= &transaction.time)
            .unwrap_or(true);
        min_amount && max_amount && account_regex && description_regex && min_time && max_time
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct Category {
    category: String,
    #[serde(default)]
    subcategories: Vec<Category>,
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
            .filter_map(|(c, transaction)| {
                (c == &self.category).then(|| {
                    total += transaction.amount.clone();
                    transaction.clone()
                })
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
pub(crate) struct Categorizer {
    category_matchers: Vec<CategoryMatcher>,
    category_hierarchy: Vec<Category>,
}

impl Categorizer {
    fn categorize(&self, transactions: Vec<Transaction>) -> CategorizedHierarchy {
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

#[derive(Debug)]
pub(crate) struct Categorized {
    category: String,
    total: BigDecimal,
    subcategories: Vec<Categorized>,
    transactions: Vec<Transaction>,
}

#[derive(Debug, Into, From)]
pub(crate) struct CategorizedHierarchy {
    categorized: Vec<Categorized>,
}
