use bigdecimal::{BigDecimal, FromPrimitive};
use test_case::test_case;

use super::format_bigdecimal;

#[test_case(BigDecimal::from_f64(1234567890.123456).unwrap(), "1,234,567,890.12")]
#[test_case(BigDecimal::from_u32(123456).unwrap(), "123,456.00")]
#[test_case(BigDecimal::from_i32(-123456).unwrap(), "-123,456.00")]
#[test_case(BigDecimal::from_u8(0).unwrap(), "0.00")]
#[test_case(BigDecimal::from_f32(0.009).unwrap(), "0.01")]
#[test_case(BigDecimal::from_f32(0.001).unwrap(), "0.00")]
#[test_case(BigDecimal::from_u8(123).unwrap(), "123.00")]
#[test_case(BigDecimal::from_i8(-123).unwrap(), "-123.00")]
#[test_case(BigDecimal::from_f64(-9876.54321).unwrap(), "-9,876.54")]
fn test_format_bigdecimal(input: BigDecimal, expected_output: &str) {
    let formatted_output = format_bigdecimal(&input);
    assert_eq!(formatted_output, expected_output);
}
