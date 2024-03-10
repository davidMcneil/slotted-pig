use anyhow::Result;
use insta::assert_yaml_snapshot;
use test_case::test_case;

use crate::{categorizer::Categorizer, transaction::Transaction};

#[test_case("tests/categorizer_empty.yaml", "tests/transactions_empty.csv", "empty"; "empty")]
#[test_case("tests/categorizer_simple.yaml", "tests/transactions_simple.csv", "simple"; "simple")]
fn test_categorizer(categorizer: &str, transactions: &str, name: &str) -> Result<()> {
    let categorizer = Categorizer::from_yaml_file(categorizer)?;
    let transactions = Transaction::from_csv_file(transactions)?;
    let (categorized, _uncategorized) = categorizer.categorize(&transactions);
    assert_yaml_snapshot!(name, categorized);
    Ok(())
}
