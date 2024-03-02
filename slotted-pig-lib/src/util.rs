use derive_more::{From, Into};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Wrapper type to allow deserializing/serializing `Regex`
#[derive(Clone, Debug, Deserialize, Into, From, Serialize)]
pub struct RegexSerde(#[serde(with = "serde_regex")] Regex);
