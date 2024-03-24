use std::sync::Arc;

use anyhow::{anyhow, Result};
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use dioxus::{html::FileEngine, prelude::*};
use log::{info, LevelFilter};
use serde::de::DeserializeOwned;
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

    let mut categorized_list_result = use_signal(|| {
        let categorized = include_str!("../../examples/categorized.yaml");
        Ok::<_, String>(
            serde_yaml::from_str::<CategorizedList>(categorized).expect("failed to parse YAML"),
        )
    });

    rsx! {
        div { class: "max-w-screen-lg mx-auto",
            div { class: "flex justify-between",
                h1 { class: "font-mono text-2xl", "Slotted Pig" }
                input {
                    r#type: "file",
                    accept: ".yaml",
                    multiple: false,
                    onchange: move |evt| {
                        async move {
                            *categorized_list_result
                                .write() = read_first_file(evt.files()).await.map_err(|e| e.to_string());
                        }
                    }
                }
            }
            match categorized_list_result.read().clone(){
                Ok(categorized_list) =>  {
                    rsx!(CategorizedList { categorized_list: categorized_list.categorized })
                }
                Err(e) => rsx!(
                    span {"{e}"}
                ),
            }
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

pub async fn read_first_file<T: DeserializeOwned>(
    file_engine: Option<Arc<dyn FileEngine>>,
) -> Result<T> {
    let file_engine = file_engine.ok_or_else(|| anyhow!("missing file engine"))?;
    let files = file_engine.files();
    let file_name = files.first().ok_or_else(|| anyhow!("missing file"))?;
    let contents = file_engine
        .read_file_to_string(file_name)
        .await
        .ok_or_else(|| anyhow!("failed to read file as string"))?;
    serde_yaml::from_str(&contents).map_err(|e| anyhow!("failed to deserialize file: {e}"))
}
