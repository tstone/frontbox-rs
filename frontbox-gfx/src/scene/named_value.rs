use std::collections::BTreeMap;

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::{Color, Inputs, Prop, VariableType};

/// A scene's named values, by name
pub type Values = BTreeMap<String, NamedValue>;

static NO_VALUES: Values = BTreeMap::new();

/// An empty set, for a scene with no named values
pub fn no_values() -> &'static Values {
  &NO_VALUES
}

/// One of a scene's named values: its type, and a prop of that type, which can be keyframed or moved in a wave as any
/// prop can. A variable points at one with `Value("name")`, as its default or from the code, such as a player's color
/// pointing at the scene's `current` or `waiting`.
///
/// In RON: `Color(Fixed(255, 230, 120, 255))`, `Float(Modulated((...)))`, `Vec2(Fixed((10.0, 4.0)))`, `Int(3)`,
/// `Text("GO")`, or `Bool(Fixed(true))`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NamedValue {
  Float(Prop<f32>),
  Vec2(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  Color(Prop<Color>),
  /// A whole number, for counts and scores too large for a `Float` to hold exactly
  Int(i64),
  Text(String),
  Bool(Prop<bool>),
}

impl NamedValue {
  pub fn variable_type(&self) -> VariableType {
    match self {
      Self::Float(_) | Self::Int(_) => VariableType::Number,
      Self::Vec2(_) => VariableType::Vec2,
      Self::Color(_) => VariableType::Color,
      Self::Text(_) => VariableType::Text,
      Self::Bool(_) => VariableType::Bool,
    }
  }

  /// Whether it can change while a scene plays, which needs the scene to have a timing
  pub fn is_animated(&self) -> bool {
    match self {
      Self::Float(prop) => prop.is_animated(),
      Self::Vec2(prop) => prop.is_animated(),
      Self::Color(prop) => prop.is_animated(),
      Self::Bool(prop) => prop.is_animated(),
      Self::Int(_) | Self::Text(_) => false,
    }
  }

  /// Adds the variables its prop reads to `inputs`
  pub fn read_inputs(&self, inputs: &mut Inputs) {
    match self {
      Self::Float(prop) => prop.read_inputs(inputs),
      Self::Vec2(prop) => prop.read_inputs(inputs),
      Self::Color(prop) => prop.read_inputs(inputs),
      Self::Bool(prop) => prop.read_inputs(inputs),
      Self::Int(_) | Self::Text(_) => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::Cycle;
  use crate::scene::value::tests::env;
  use crate::scene::{Env, Key, Keyframes, Timing, Value, Variables, file};

  fn values() -> Values {
    file::load(r#"{"current": Float(Fixed(2.0)), "waiting": Float(Variable("dim")), "loop": Float(Variable("loop")), "name": Text("TOM")}"#).unwrap()
  }

  fn at(prop: &Prop<f32>, variables: &Variables, values: &Values) -> f32 {
    prop.resolve(&Env {
      values,
      ..env(0.0, variables)
    })
  }

  #[test]
  fn a_variable_pointing_at_a_value_is_the_values_prop() {
    let variables = Variables::from([("size".into(), Value::Named("current".into()))]);
    assert_eq!(
      at(&Prop::Variable("size".into()), &variables, &values()),
      2.0
    );
  }

  #[test]
  fn a_value_can_read_a_variable() {
    let variables = Variables::from([
      ("size".into(), Value::Named("waiting".into())),
      ("dim".into(), Value::Float(0.5)),
    ]);
    assert_eq!(
      at(&Prop::Variable("size".into()), &variables, &values()),
      0.5
    );
  }

  #[test]
  fn a_missing_value_or_one_of_another_type_is_the_types_default() {
    let variables = Variables::from([
      ("a".into(), Value::Named("nope".into())),
      ("b".into(), Value::Named("name".into())),
    ]);
    assert_eq!(at(&Prop::Variable("a".into()), &variables, &values()), 0.0);
    assert_eq!(at(&Prop::Variable("b".into()), &variables, &values()), 0.0);
  }

  #[test]
  fn a_value_that_points_back_at_itself_stops() {
    let variables = Variables::from([("loop".into(), Value::Named("loop".into()))]);
    assert_eq!(
      at(&Prop::Variable("loop".into()), &variables, &values()),
      0.0
    );
  }

  #[test]
  fn a_value_can_be_keyframed() {
    let keys = Keyframes::new(
      vec![
        Key::new(0.0, 0.0, Default::default()),
        Key::new(1.0, 10.0, Default::default()),
      ],
      Cycle::Once,
    );
    let values = Values::from([("pulse".into(), NamedValue::Float(Prop::Keyframes(keys)))]);
    let variables = Variables::from([("size".into(), Value::Named("pulse".into()))]);
    let env = Env {
      values: &values,
      timing: Some(Timing::new(1.0, Cycle::Once)),
      ..env(0.5, &variables)
    };
    assert_eq!(Prop::<f32>::Variable("size".into()).resolve(&env), 5.0);
    assert!(values["pulse"].is_animated());
  }

  #[test]
  fn a_variable_reads_from_ron_as_a_value_name() {
    let value: Value = file::load(r#"Value("current")"#).unwrap();
    assert_eq!(value, Value::Named("current".into()));
  }
}
