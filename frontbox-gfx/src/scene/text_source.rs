use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop, Value};

/// Where a [`ShapeNode::Text`]'s text comes from
///
/// In RON: `Fixed("BALL 1")`, `Param("player")`, or `Number((value: Param("score"), format: Grouped))`.
///
/// [`ShapeNode::Text`]: crate::scene::ShapeNode::Text
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextSource {
  Fixed(String),
  /// Read from a text parameter. A number parameter is written out in digits, and a missing one, or one of another type,
  /// is no text.
  Param(String),
  /// A number written out in `format`, such as a score from a parameter or a bonus counted up on keyframes
  Number(NumberText),
}

impl From<&str> for TextSource {
  fn from(text: &str) -> Self {
    Self::Fixed(text.into())
  }
}

impl From<String> for TextSource {
  fn from(text: String) -> Self {
    Self::Fixed(text)
  }
}

impl TextSource {
  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::Fixed(_) | Self::Param(_) => false,
      Self::Number(number) => number.value.is_animated(),
    }
  }

  pub(crate) fn resolve<'a>(&'a self, env: &'a Env) -> Cow<'a, str> {
    match self {
      Self::Fixed(text) => Cow::Borrowed(text),
      Self::Param(name) => match env.param(name) {
        Some(Value::Text(text)) => Cow::Borrowed(text),
        Some(Value::Int(number)) => Cow::Owned(number.to_string()),
        Some(Value::Float(number)) => Cow::Owned(number.to_string()),
        _ => Cow::Borrowed(""),
      },
      Self::Number(number) => Cow::Owned(number.format.write(number.resolve(env))),
    }
  }
}

/// A number for [`TextSource::Number`], and how it's written out
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumberText {
  pub value: Prop<f32>,
  #[serde(default)]
  pub format: NumberFormat,
}

impl NumberText {
  /// A parameter is read here rather than through `value`, as an `Int` score can be too large for an `f32` to hold
  /// exactly
  fn resolve(&self, env: &Env) -> i64 {
    match &self.value {
      Prop::Param(name) => match env.param(name) {
        Some(Value::Int(number)) => *number,
        Some(Value::Float(number)) => *number as i64,
        _ => 0,
      },
      value => value.resolve(env) as i64,
    }
  }
}

/// How a number is written out. Each truncates it to a whole number first.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NumberFormat {
  /// With commas between the thousands: 12,942
  #[default]
  Grouped,
  /// Digits only: 12942
  Plain,
  /// In thousands (k), millions (M), billions (B), and so on, with as many decimals as fit in `length` characters,
  /// truncated: 12,942 is "12.94k" in 6, "12.9k" in 5, and "12k" in 4 or fewer. Trailing zeros are dropped, and under a
  /// thousand it's the digits.
  Abbreviated { length: u8 },
}

impl NumberFormat {
  pub fn write(self, number: i64) -> String {
    match self {
      Self::Grouped => grouped(number),
      Self::Plain => number.to_string(),
      Self::Abbreviated { length } => abbreviated(number, length.into()),
    }
  }
}

fn grouped(number: i64) -> String {
  let digits = number.unsigned_abs().to_string();
  let mut written = String::with_capacity(digits.len() + digits.len() / 3 + 1);
  if number < 0 {
    written.push('-');
  }
  for (i, digit) in digits.chars().enumerate() {
    if i > 0 && (digits.len() - i) % 3 == 0 {
      written.push(',');
    }
    written.push(digit);
  }
  written
}

fn abbreviated(number: i64, length: usize) -> String {
  const SUFFIXES: [&str; 7] = ["", "k", "M", "B", "T", "P", "E"];
  let sign = if number < 0 { "-" } else { "" };
  let magnitude = number.unsigned_abs();
  let mut scale = 1u64;
  let mut suffix = 0;
  while magnitude / scale >= 1000 {
    scale *= 1000;
    suffix += 1;
  }
  let (whole, part) = (magnitude / scale, magnitude % scale);
  let written = |decimals: u32| {
    // integer math, as truncating a float can land a digit short
    let digits = (part as u128 * 10u128.pow(decimals) / scale as u128) as u64;
    let digits = format!("{digits:0width$}", width = decimals as usize);
    let digits = digits.trim_end_matches('0');
    let point = if digits.is_empty() { "" } else { "." };
    format!("{sign}{whole}{point}{digits}{}", SUFFIXES[suffix])
  };
  if suffix == 0 {
    return written(0);
  }
  (1..=2).rev().map(written).find(|text| text.len() <= length).unwrap_or_else(|| written(0))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::scene::value::tests::env;
  use crate::scene::{Key, Keyframes, Params};

  #[test]
  fn grouped_puts_commas_between_thousands() {
    assert_eq!(NumberFormat::Grouped.write(0), "0");
    assert_eq!(NumberFormat::Grouped.write(942), "942");
    assert_eq!(NumberFormat::Grouped.write(12_942), "12,942");
    assert_eq!(NumberFormat::Grouped.write(1_000_000), "1,000,000");
    assert_eq!(NumberFormat::Grouped.write(-123_456), "-123,456");
  }

  #[test]
  fn abbreviated_keeps_the_decimals_that_fit() {
    let abbreviated = |number, length| NumberFormat::Abbreviated { length }.write(number);
    assert_eq!(abbreviated(12_942, 6), "12.94k");
    assert_eq!(abbreviated(12_942, 5), "12.9k");
    assert_eq!(abbreviated(12_942, 3), "12k");
    assert_eq!(abbreviated(12_942, 1), "12k");
    assert_eq!(abbreviated(942, 2), "942");
    assert_eq!(abbreviated(1_500_000, 4), "1.5M");
    assert_eq!(abbreviated(2_750_000_000, 5), "2.75B");
    assert_eq!(abbreviated(-12_942, 6), "-12.9k");
  }

  #[test]
  fn abbreviated_truncates_without_float_error() {
    // 0.29 * 100 is 28.999... in floating point
    assert_eq!(NumberFormat::Abbreviated { length: 5 }.write(1_290), "1.29k");
    assert_eq!(NumberFormat::Abbreviated { length: 5 }.write(1_999), "1.99k");
  }

  #[test]
  fn abbreviated_drops_trailing_zeros() {
    assert_eq!(NumberFormat::Abbreviated { length: 5 }.write(1_000_000), "1M");
    assert_eq!(NumberFormat::Abbreviated { length: 5 }.write(1_204), "1.2k");
  }

  #[test]
  fn param_text_is_read_and_numbers_are_written_out() {
    let params = Params::from([("name".into(), Value::Text("TOM".into())), ("ball".into(), Value::Int(3))]);
    let env = env(0.0, &params);
    assert_eq!(TextSource::Param("name".into()).resolve(&env), "TOM");
    assert_eq!(TextSource::Param("ball".into()).resolve(&env), "3");
    assert_eq!(TextSource::Param("missing".into()).resolve(&env), "");
  }

  #[test]
  fn a_number_param_is_exact_past_what_a_float_holds() {
    let params = Params::from([("score".into(), Value::Int(123_456_789))]);
    let number = TextSource::Number(NumberText { value: Prop::Param("score".into()), format: NumberFormat::Grouped });
    assert_eq!(number.resolve(&env(0.0, &params)), "123,456,789");
  }

  #[test]
  fn a_number_counts_up_on_keyframes() {
    let keys = Keyframes::new(vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(1.0, 1000.0, Curve::Linear)], Cycle::Once);
    let number = TextSource::Number(NumberText { value: Prop::Keyframes(keys), format: NumberFormat::Grouped });
    assert!(number.is_animated());
    assert_eq!(number.resolve(&env(0.5, &Params::new())), "500");
  }
}
