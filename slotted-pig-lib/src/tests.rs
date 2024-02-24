use anyhow::Result;
use insta::assert_json_snapshot;
use test_case::test_case;

use crate::{categorizer::Categorizer, transaction::Transaction};

#[test_case("tests/categorizer_empty.json", "tests/transactions_empty.csv", "empty"; "empty")]
#[test_case("tests/categorizer_simple.json", "tests/transactions_simple.csv", "simple"; "simple")]
fn test_categorizer(categorizer: &str, transactions: &str, name: &str) -> Result<()> {
    let categorizer = Categorizer::from_json_file(categorizer)?;
    let transactions = Transaction::from_csv_file(transactions)?;
    assert_json_snapshot!(name, categorizer.categorize(&transactions));
    Ok(())
}
