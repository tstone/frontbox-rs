use std::fmt;

use frontbox_gfx::scene::{Value, VariableType};

use crate::Scene;

/// The variables a system sends the scenes it plays, and the named values it points them at, so a scene someone wrote
/// for it can be checked against them with [`Sends::check`]
///
/// ```rust
/// # use frontbox_gfx::scene::VariableType;
/// # use frontbox_scene::*;
/// const POINTS: Sends = Sends {
///   variables: &[
///     SentVariable { name: "score", sent_as: VariableType::Number, values: &[], about: "The current player's score" },
///     SentVariable { name: "player_1_color", sent_as: VariableType::Color, values: &["current", "waiting"], about: "Player 1's score's color" },
///   ],
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Sends {
  pub variables: &'static [SentVariable],
}

impl Sends {
  /// What in `scene` doesn't fit what's sent. A scene doesn't have to read everything that's sent, so that isn't
  /// listed.
  pub fn check(&self, scene: &Scene) -> Vec<Mismatch> {
    let mut mismatches = Vec::new();
    for (name, types) in &scene.inputs().variables {
      let missing = |value: &str, by| Mismatch::MissingValue { variable: name.clone(), value: value.into(), by };
      let named = |value: &str| scene.values.get(value).map(|named| (named.variable_type(), Given::Value(value.into())));
      let mut given = Vec::new();
      match (self.variables.iter().find(|variable| variable.name == name), scene.variables.get(name)) {
        (Some(sent), _) => {
          given.push((sent.sent_as, Given::Sent));
          for value in sent.values {
            match named(value) {
              Some(type_and_value) => given.push(type_and_value),
              None => mismatches.push(missing(value, Given::Sent)),
            }
          }
        }
        (None, Some(Value::Named(value))) => match named(value) {
          Some(type_and_value) => given.push(type_and_value),
          None => mismatches.push(missing(value, Given::Default)),
        },
        (None, Some(default)) => given.extend(VariableType::of(default).map(|of| (of, Given::Default))),
        (None, None) => types.iter().for_each(|&read_as| mismatches.push(Mismatch::NotSent { variable: name.clone(), read_as })),
      }
      for &read_as in types {
        for (of, by) in given.iter().filter(|(of, _)| !read_as.reads(*of)) {
          mismatches.push(Mismatch::WrongType { variable: name.clone(), read_as, given: *of, by: by.clone() });
        }
      }
    }
    mismatches
  }

  /// Logs a warning for each [`Mismatch`] in scene `name`. The scene still plays, and each variable that doesn't fit
  /// draws as its type's zero value.
  pub fn log_mismatches(&self, name: &str, scene: &Scene) {
    for mismatch in self.check(scene) {
      let sent = match mismatch {
        Mismatch::NotSent { .. } | Mismatch::WrongType { by: Given::Sent, .. } => {
          format!(" Variables sent: {}.", self.variables.iter().map(|variable| variable.name).collect::<Vec<_>>().join(", "))
        }
        Mismatch::MissingValue { .. } => format!(" The scene's values: {}.", scene.values.keys().cloned().collect::<Vec<_>>().join(", ")),
        _ => String::new(),
      };
      log::warn!(target: "frontbox::scene", "Scene \"{name}\": {mismatch}.{sent}");
    }
  }
}

/// A variable a system sends the scenes it plays
#[derive(Debug, Clone, Copy)]
pub struct SentVariable {
  pub name: &'static str,
  /// The type of value sent
  pub sent_as: VariableType,
  /// The scene's named values it's pointed at, if it's pointed at any
  pub values: &'static [&'static str],
  /// What it means, for someone writing a scene for the system
  pub about: &'static str,
}

/// Something a scene reads that doesn't fit what the system playing it sends, from [`Sends::check`]
#[derive(Debug, Clone, PartialEq)]
pub enum Mismatch {
  /// A variable it reads isn't sent, and the scene gives it no default. Most likely a typo, or a variable that was
  /// renamed.
  NotSent { variable: String, read_as: VariableType },
  /// A variable is sent, given a default, or pointed at a named value, as a type it isn't read as
  WrongType { variable: String, read_as: VariableType, given: VariableType, by: Given },
  /// A variable is pointed at a named value the scene doesn't have
  MissingValue { variable: String, value: String, by: Given },
}

/// Where a variable's value comes from
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Given {
  /// Sent by the system
  Sent,
  /// The scene's default
  Default,
  /// One of the scene's named values
  Value(String),
}

impl fmt::Display for Mismatch {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NotSent { variable, read_as } => {
        write!(f, "variable \"{variable}\" is read as {read_as}, but isn't sent and has no default")
      }
      Self::WrongType { variable, read_as, given, by } => {
        let by = match by {
          Given::Sent => format!("is sent as {given}"),
          Given::Default => format!("its default is {given}"),
          Given::Value(value) => format!("value \"{value}\" is {given}"),
        };
        write!(f, "variable \"{variable}\" is read as {read_as}, but {by}")
      }
      Self::MissingValue { variable, value, by } => {
        let by = if *by == Given::Default { "its default points" } else { "it's pointed" };
        write!(f, "variable \"{variable}\" reads as zero where {by} at value \"{value}\", which the scene doesn't have")
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::scene::Value;

  use super::*;

  const SENDS: Sends = Sends {
    variables: &[
      SentVariable { name: "score", sent_as: VariableType::Number, values: &[], about: "" },
      SentVariable { name: "name", sent_as: VariableType::Text, values: &[], about: "" },
      SentVariable { name: "player_color", sent_as: VariableType::Color, values: &["current", "waiting"], about: "" },
    ],
  };

  fn scene(values: &str, layers: &str) -> Scene {
    Scene::load(&format!("Scene(version: 2, values: {{{values}}}, layers: [{layers}])")).unwrap()
  }

  fn text(source: &str) -> String {
    format!(r#"Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(0, 0, 0, 255))), region: (base: Text(text: {source}, font: "f", size: Fixed(8.0), width: Fixed(10.0), height: Fixed(8.0)))))"#)
  }

  fn solid(color: &str) -> String {
    format!(r#"Display(display: "Dmd", layer: (fill: (base: Solid({color}))))"#)
  }

  const VALUES: &str = r#""current": Color(Fixed(255, 230, 120, 255)), "waiting": Color(Fixed(150, 150, 150, 255))"#;

  #[test]
  fn a_scene_reading_only_what_is_sent_fits() {
    let layers = [text(r#"Number(value: Variable("score"))"#), text(r#"Variable("score")"#), solid(r#"Variable("player_color")"#)];
    assert_eq!(SENDS.check(&scene(VALUES, &layers.join(", "))), []);
  }

  #[test]
  fn a_variable_that_isnt_sent_fits_if_the_scene_gives_it_a_default() {
    let mut scene = scene(VALUES, &solid(r#"Variable("accent")"#));
    assert_eq!(SENDS.check(&scene), [Mismatch::NotSent { variable: "accent".into(), read_as: VariableType::Color }]);
    scene.variables.insert("accent".into(), Value::Color(Default::default()));
    assert_eq!(SENDS.check(&scene), []);
    scene.variables.insert("accent".into(), Value::named("current"));
    assert_eq!(SENDS.check(&scene), []);
  }

  #[test]
  fn a_variable_read_as_another_type_doesnt_fit() {
    let mut scene = scene(VALUES, &[solid(r#"Variable("score")"#), solid(r#"Variable("accent")"#)].join(", "));
    scene.variables.insert("accent".into(), Value::Float(1.0));
    assert_eq!(SENDS.check(&scene), [
      Mismatch::WrongType { variable: "accent".into(), read_as: VariableType::Color, given: VariableType::Number, by: Given::Default },
      Mismatch::WrongType { variable: "score".into(), read_as: VariableType::Color, given: VariableType::Number, by: Given::Sent },
    ]);
  }

  #[test]
  fn text_doesnt_fit_where_a_number_is_read() {
    let scene = scene("", &text(r#"Number(value: Variable("name"))"#));
    assert_eq!(SENDS.check(&scene), [Mismatch::WrongType { variable: "name".into(), read_as: VariableType::Number, given: VariableType::Text, by: Given::Sent }]);
  }

  #[test]
  fn values_the_scene_lacks_or_of_another_type_dont_fit() {
    let scene = scene(r#""current": Float(Fixed(1.0))"#, &solid(r#"Variable("player_color")"#));
    assert_eq!(SENDS.check(&scene), [
      Mismatch::MissingValue { variable: "player_color".into(), value: "waiting".into(), by: Given::Sent },
      Mismatch::WrongType { variable: "player_color".into(), read_as: VariableType::Color, given: VariableType::Number, by: Given::Value("current".into()) },
    ]);
  }

  #[test]
  fn a_default_pointing_at_a_missing_value_doesnt_fit() {
    let mut scene = scene("", &solid(r#"Variable("accent")"#));
    scene.variables.insert("accent".into(), Value::named("gone"));
    assert_eq!(SENDS.check(&scene), [Mismatch::MissingValue { variable: "accent".into(), value: "gone".into(), by: Given::Default }]);
  }

  #[test]
  fn a_value_reading_a_variable_is_checked_too() {
    let scene = scene(r#""current": Color(Variable("glow"))"#, "");
    assert_eq!(SENDS.check(&scene), [Mismatch::NotSent { variable: "glow".into(), read_as: VariableType::Color }]);
  }
}
