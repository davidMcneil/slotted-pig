use dioxus::prelude::*;
use log::LevelFilter;
use slotted_pig_lib::{
    categorizer::{CategorizedList, Categorizer},
    transaction::Transaction,
};

const TRANSACTIONS: &str = "amount,account,description,time
-10,credit card,store1,2024-02-24T20:10:59Z
-20,credit card,store2,2024-02-23T20:10:59Z
5,checking,paycheck,2024-02-01T20:10:59Z";

const CATEGORIZER: &str = "matchers:
- category: store
  description: 'store.*'
- category: paycheck
  min: 0

hierarchy:
- category: expenses
  subcategories:
    - category: store
- category: income
  subcategories:
    - category: paycheck";

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");

    #[cfg(not(target_arch = "wasm32"))]
    dioxus_desktop::launch(App);
    #[cfg(target_arch = "wasm32")]
    dioxus_web::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    let transactions = use_state(cx, || Transaction::from_csv_buffer(TRANSACTIONS).unwrap());
    let categorizer = use_state(cx, || Categorizer::from_yaml_buffer(CATEGORIZER).unwrap());
    let categorized = use_state(cx, CategorizedList::default);

    render!(
        div { "Header" }
        TransactionList { transactions: transactions }
        Categorizer { _categorizer: categorizer }
        Categorized { categorized: categorized }
        button { onclick: move |_| { categorized.set(categorizer.get().categorize(transactions.get())) },
            "Categorize"
        }
        Counter {}
        Counter {}
    )
}

#[component]
fn TransactionList<'a>(cx: Scope, transactions: &'a UseState<Vec<Transaction>>) -> Element {
    let ts = transactions.get();
    render!(
        div { "Transactions: {ts.len()}" }
        ts.iter().map(|t| rsx!(Transaction {transaction: t}))
    )
}

#[component]
fn Transaction<'a>(cx: Scope, transaction: &'a Transaction) -> Element {
    render!( div { "{transaction.amount} | {transaction.time}" } )
}

#[component]
fn Categorizer<'a>(cx: Scope, _categorizer: &'a UseState<Categorizer>) -> Element {
    render!( div { "Categorizer" } )
}

#[component]
fn Categorized<'a>(cx: Scope, categorized: &'a UseState<CategorizedList>) -> Element {
    let categorized = categorized.get();
    render!(
        div { "Categorized" }
        categorized.categorized.iter().map(|c| rsx!(
            div {"{c.category} | {c.total}"}
        ))
    )
}

#[component]
fn Counter(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);
    render!(
        div { "Count: {count}" }
        button { onclick: move |_| { count += 1 }, "Increment" }
        button { onclick: move |_| { count -= 1 }, "Decrement" }
    )
}
