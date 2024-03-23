use bigdecimal::{BigDecimal, Signed};
use derive_more::{From, Into};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

/// Wrapper type to allow deserializing/serializing `Regex`
#[derive(Clone, Debug, Deserialize, Into, From, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegexSerde(#[serde(with = "serde_regex")] Regex);

/// Nicely format a bigdecimal value with two decimal places and commas
pub fn format_bigdecimal(number: &BigDecimal) -> String {
    let decimals = 2u8;
    let number = number.round(decimals.into());

    // Extract integer and fractional parts
    let negative = number.is_negative();
    let s = number.abs().to_string();
    let (integer, fractional) = s.split_once('.').unwrap_or((&s, ""));

    let mut formatted = String::new();

    // Insert commas every three digits
    let mut count = 0;
    for c in integer.chars().rev() {
        if count == 3 {
            formatted.insert(0, ',');
            count = 0;
        }
        formatted.insert(0, c);
        count += 1;
    }

    // Insert sign
    if negative {
        formatted.insert(0, '-');
    }

    // Insert decimal point and fractional part
    formatted.push('.');
    formatted.push_str(&fractional[..std::cmp::min(decimals.into(), fractional.len())]);
    let zero_fill = match fractional.len() {
        0 => "00",
        1 => "0",
        _ => "",
    };
    formatted.push_str(zero_fill);

    formatted
}
