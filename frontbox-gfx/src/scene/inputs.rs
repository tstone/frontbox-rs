use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::scene::{EnvelopeKey, Value};

/// What a scene's layers read from outside them: variables from the code playing them, and what their external
/// envelopes follow. Each layer adds what it reads with [`Layer::read_inputs`](crate::scene::Layer::read_inputs).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Inputs {
  /// Each variable by name, with every type it's read as
  pub variables: BTreeMap<String, BTreeSet<VariableType>>,
  /// Each [`ExternalEnvelope`](crate::scene::ExternalEnvelope)'s source and slew, which the scene's assets prepare it by
  pub envelopes: BTreeSet<EnvelopeKey>,
  /// The layers whose regions its [`Sweep`](crate::scene::Sweep)s follow, by name
  pub regions: BTreeSet<String>,
}

impl Inputs {
  pub fn new() -> Self {
    Self::default()
  }

  pub(crate) fn variable(&mut self, name: &str, read_as: VariableType) {
    self
      .variables
      .entry(name.into())
      .or_default()
      .insert(read_as);
  }

  pub(crate) fn envelope(&mut self, key: EnvelopeKey) {
    self.envelopes.insert(key);
  }

  pub(crate) fn region(&mut self, layer: &str) {
    self.regions.insert(layer.into());
  }
}

/// The type a variable is read as
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VariableType {
  /// A `Float` or an `Int`
  Number,
  Vec2,
  Color,
  /// A `Text`, or a number written out in digits
  Text,
  /// A `Bool`, or a number, which is true from 0.5 up
  Bool,
}

impl VariableType {
  /// The type of a value. `None` for one pointing at a named value, which has the named value's type.
  pub fn of(value: &Value) -> Option<Self> {
    match value {
      Value::Float(_) | Value::Int(_) => Some(Self::Number),
      Value::Vec2(_) => Some(Self::Vec2),
      Value::Color(_) => Some(Self::Color),
      Value::Text(_) => Some(Self::Text),
      Value::Bool(_) => Some(Self::Bool),
      Value::Named(_) => None,
    }
  }

  /// Whether a variable read as this type can be one of type `given`. Text can be a number, written out in digits, and
  /// true or false can be a number, true from 0.5 up.
  pub fn reads(self, given: VariableType) -> bool {
    self == given || matches!((self, given), (Self::Text | Self::Bool, Self::Number))
  }
}

impl fmt::Display for VariableType {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Number => "a number",
      Self::Vec2 => "a position",
      Self::Color => "a color",
      Self::Text => "text",
      Self::Bool => "true or false",
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::Layer;
  use crate::scene::file;

  fn inputs(layer: &str) -> Inputs {
    let layer: Layer = file::load(layer).unwrap();
    let mut inputs = Inputs::new();
    layer.read_inputs(&mut inputs);
    inputs
  }

  #[test]
  fn variables_are_listed_with_the_type_they_are_read_as() {
    let inputs = inputs(
      r#"(
        fill: (base: Solid(Variable("color")), adjust: [Opacity(Variable("fade"))]),
        region: (base: Text(text: Number(value: Variable("score")), font: "f", size: Fixed(8.0), width: Fixed(10.0), height: Variable("fade")), transform: [Translate(Variable("at"))]),
      )"#,
    );
    let types = |name: &str| inputs.variables[name].iter().copied().collect::<Vec<_>>();
    assert_eq!(types("color"), [VariableType::Color]);
    assert_eq!(types("fade"), [VariableType::Number]);
    assert_eq!(types("score"), [VariableType::Number]);
    assert_eq!(types("at"), [VariableType::Vec2]);
    assert_eq!(inputs.variables.len(), 4);
  }

  #[test]
  fn text_read_from_a_variable_is_text() {
    let inputs = inputs(
      r#"(fill: (base: Solid(Fixed(0, 0, 0, 255))), region: (base: Text(text: Variable("name"), font: "f", size: Fixed(8.0), width: Fixed(10.0), height: Fixed(8.0))))"#,
    );
    assert_eq!(
      inputs.variables["name"].iter().collect::<Vec<_>>(),
      [&VariableType::Text]
    );
  }

  #[test]
  fn variables_inside_other_props_are_found() {
    let inputs = inputs(
      r#"(fill: (base: Solid(Fixed(0, 0, 0, 255))),
        region: (base: Circle(radius: Variable("orbit")), transform: [Translate(Sweep(layer: "track", position: Variable("lap")))]))"#,
    );
    assert_eq!(
      inputs.variables.keys().collect::<Vec<_>>(),
      ["lap", "orbit"]
    );
    assert_eq!(inputs.regions.iter().collect::<Vec<_>>(), ["track"]);
  }

  #[test]
  fn a_file_from_before_variables_says_param() {
    let inputs = inputs(
      r#"(fill: (base: Solid(Param("color"))), region: (base: Text(text: Param("name"), font: "f", size: Fixed(8.0), width: Fixed(10.0), height: Fixed(8.0))))"#,
    );
    assert_eq!(
      inputs.variables.keys().collect::<Vec<_>>(),
      ["color", "name"]
    );
  }

  #[test]
  fn a_variable_type_reads_the_types_it_can_be_read_from() {
    assert!(VariableType::Number.reads(VariableType::Number));
    assert!(VariableType::Text.reads(VariableType::Number));
    assert!(!VariableType::Number.reads(VariableType::Text));
    assert_eq!(VariableType::of(&Value::Named("current".into())), None);
  }
}
