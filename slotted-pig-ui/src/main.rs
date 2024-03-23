use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use log::{info, LevelFilter};
use slotted_pig_lib::{
    categorizer::{Categorized, CategorizedChildren, CategorizedList},
    transaction::Transaction,
    util::format_bigdecimal,
};

fn main() {
    dioxus_logger::init(LevelFilter::Info).expect("failed to init logger");

    #[cfg(feature = "desktop")]
    {
        let cfg = dioxus::desktop::Config::new()
            .with_custom_head(r#"<link rel="stylesheet" href="assets/tailwind.css">"#.to_string());
        LaunchBuilder::desktop().with_cfg(cfg).launch(App)
    }

    #[cfg(feature = "web")]
    LaunchBuilder::web().launch(App)
}

#[component]
fn App() -> Element {
    info!("slotted-pig");

    let categorized_list = use_signal(|| {
        let categorized = include_str!("../../personal/categorized.yaml");
        serde_yaml::from_str::<CategorizedList>(categorized).expect("Failed to parse YAML")
    });

    let categorized_list = categorized_list().clone().categorized;

    rsx! {
        div { class: "max-w-screen-lg mx-auto px-4",
            h1 { class: "font-mono text-lg", "Slotted Pig" }
            CategorizedList { categorized_list }
        }
    }
}

#[component]
fn CategorizedList(categorized_list: Vec<Categorized>) -> Element {
    rsx!(
        ul { class: "list-disc pl-4",
            for categorized in categorized_list {
                li {
                    Categorized { categorized }
                }
            }
        }
    )
}

#[component]
fn Categorized(categorized: Categorized) -> Element {
    let mut hidden = use_signal(|| true);

    let Categorized {
        category,
        count,
        total,
        children,
        ..
    } = categorized;

    rsx!(
        div { class: "hover:cursor-pointer", onclick: move |_| *hidden.write() = !hidden(),
            span { class: "font-mono text-base px-1", "{category}" }
            span { class: "font-mono text-sm px-1", "[{count}]" }
            Amount { amount: total }
        }
        div { class: if hidden() { "hidden" } else { "" },
            match children {
                CategorizedChildren::Transactions(transactions) => {
                    rsx!(Transactions{transactions})
                },
                CategorizedChildren::Subcategories(categorized_list) => {
                    rsx!(CategorizedList{categorized_list})
                },
            }
        }
    )
}

#[component]
fn Transactions(transactions: Vec<Transaction>) -> Element {
    rsx!(
        ul { class: "list-disc pl-4",
            for transaction in transactions {
                li {
                    Transaction { transaction }
                }
            }
        }
    )
}

#[component]
fn Transaction(transaction: Transaction) -> Element {
    let Transaction {
        amount,
        time,
        description,
        ..
    } = transaction;
    rsx!(
        Amount { amount: amount }
        Time { time }
        span { class: "font-mono text-xs", "{description}" }
    )
}

#[component]
fn Amount(amount: BigDecimal) -> Element {
    let amount = format_bigdecimal(&amount);
    rsx!( span { class: "font-mono text-sm px-1", "{amount}" } )
}

#[component]
fn Time(time: DateTime<Utc>) -> Element {
    let time = time.format("%Y-%m-%d");
    rsx!( span { class: "font-mono text-sm px-1", "{time}" } )
}
