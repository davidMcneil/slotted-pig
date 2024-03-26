# Slotted Pig

Keep track of your funds, no pork barrel spending.

## Dev

Print transactions
> cargo run --bin slotted-pig-cli -- --transaction-parser-path examples/transaction_parser.yaml --transaction-path-pattern "examples/*.csv" --categorizer-path examples/categorizer.yaml transactions

Output categorized transactions to `examples/categorized.yaml`
> cargo run --bin slotted-pig-cli -- --transaction-parser-path examples/transaction_parser.yaml --transaction-path-pattern "examples/*.csv" --categorizer-path examples/categorizer.yaml categorize --transaction-sort absolute_amount_descending --category-sort absolute_total_descending  > examples/categorized.yaml

From `slotted-pig-ui` run the following commands for ui development

Run on the desktop
> dx serve --hot-reload --features desktop --platform desktop

Run for the web. First you have to comment out `base_path = "slotted-pig"` in Dioxus.toml
> dx serve --hot-reload --features web --platform web

If you are editing CSS you need to run the below to have `assets/tailwind.css` automatically updated.
> npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch

Create a build for the web
> dx build --release --platform web --features web

## UI

### By Category
* start end time select
* Hierarchy tree view
    * expand and collapse subcategories and piechart showing ratios

### By Time Period
* time period select
* category selections
* bar chart with the totals for the chosen time period