use std::{collections::HashSet, hash::Hash};

use derive_more::{From, Into};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Wrapper type to allow deserializing/serializing `Regex`
#[derive(Clone, Debug, Deserialize, Into, From, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegexSerde(#[serde(with = "serde_regex")] Regex);

/// Convert a vector to two hashsets: one of unique elements and one of duplicates
pub fn vec_to_hashsets<T: Hash + Eq>(vec: impl Iterator<Item = T>) -> (HashSet<T>, HashSet<T>) {
    let mut unique_set: HashSet<T> = HashSet::new();
    let mut duplicate_set: HashSet<T> = HashSet::new();
    for item in vec {
        if unique_set.contains(&item) {
            duplicate_set.insert(item);
        } else {
            unique_set.insert(item);
        }
    }
    (unique_set, duplicate_set)
}
