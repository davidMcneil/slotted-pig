use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Basic transaction type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Transaction {
    pub amount: BigDecimal,
    pub account: String,
    pub description: String,
    pub time: DateTime<Utc>,
}
