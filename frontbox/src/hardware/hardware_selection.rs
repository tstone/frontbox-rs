use std::any::TypeId;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareSelection {
  Name(&'static str),
  Group(Vec<&'static str>),
  Tag(TypeId),
  And(Box<HardwareSelection>, Box<HardwareSelection>),
  Or(Box<HardwareSelection>, Box<HardwareSelection>),
}

impl HardwareSelection {
  /// Creates a selection that matches any switch/driver with the specified name.
  pub fn name(name: &'static str) -> Self {
    Self::Name(name)
  }

  /// Creates a selection that matches any switch/driver with the specified tag type.
  pub fn tag<T: HardwareTag + 'static>() -> Self {
    Self::Tag(TypeId::of::<T>())
  }

  /// Creates a selection that matches any of the provided names.
  pub fn group(names: Vec<&'static str>) -> Self {
    Self::Group(names)
  }

  /// Creates a selection that matches if both sub-selections match.
  pub fn and(left: Self, right: Self) -> Self {
    Self::And(Box::new(left), Box::new(right))
  }

  /// Creates a selection that matches if either sub-selection matches.
  pub fn or(left: Self, right: Self) -> Self {
    Self::Or(Box::new(left), Box::new(right))
  }

  /// Sums up multiple selections with OR logic. Panics if the input is empty.
  pub fn any_of(selections: Vec<Self>) -> Self {
    selections.into_iter().reduce(Self::or).unwrap()
  }

  /// Sums up multiple selections with AND logic. Panics if the input is empty.
  pub fn all_of(selections: Vec<Self>) -> Self {
    selections.into_iter().reduce(Self::and).unwrap()
  }

  pub fn matches_switch(&self, switch: &Switch) -> bool {
    match self {
      Self::Name(name) => switch.name == *name,
      Self::Group(names) => names.contains(&switch.name),
      Self::Tag(tag) => switch.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_switch(switch) && right.matches_switch(switch),
      Self::Or(left, right) => left.matches_switch(switch) || right.matches_switch(switch),
    }
  }

  pub fn matches_driver(&self, driver: &Driver) -> bool {
    match self {
      Self::Name(name) => driver.name == *name,
      Self::Group(names) => names.contains(&driver.name),
      Self::Tag(tag) => driver.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_driver(driver) && right.matches_driver(driver),
      Self::Or(left, right) => left.matches_driver(driver) || right.matches_driver(driver),
    }
  }

  pub fn matches_illumination(&self, illumination: &AddressableIllumination) -> bool {
    match self {
      Self::Name(name) => illumination.name() == *name,
      Self::Group(names) => names.contains(&illumination.name()),
      Self::Tag(tag) => illumination.has_typed_tag(*tag),
      Self::And(left, right) => {
        left.matches_illumination(illumination) && right.matches_illumination(illumination)
      }
      Self::Or(left, right) => {
        left.matches_illumination(illumination) || right.matches_illumination(illumination)
      }
    }
  }

  pub fn get_switches<'a>(&self, ctx: &'a Context) -> Vec<&'a Switch> {
    ctx.switches.by_selection(&self)
  }

  pub fn get_drivers<'a>(&self, ctx: &'a Context) -> Vec<&'a Driver> {
    ctx.drivers.by_selection(&self)
  }

  pub fn get_illuminations<'a>(&self, ctx: &'a Context) -> Vec<&'a AddressableIllumination> {
    ctx
      .illuminations
      .values()
      .filter(|illum| self.matches_illumination(illum))
      .collect()
  }
}

pub trait HardwareTagExt {
  fn get_switches<'a>(&self, ctx: &'a Context) -> Vec<&'a Switch>;
  fn get_drivers<'a>(&self, ctx: &'a Context) -> Vec<&'a Driver>;
  fn get_illuminations<'a>(&self, ctx: &'a Context) -> Vec<&'a AddressableIllumination>;
}

impl HardwareTagExt for Option<HardwareSelection> {
  fn get_switches<'a>(&self, ctx: &'a Context) -> Vec<&'a Switch> {
    self
      .as_ref()
      .map(|sel| sel.get_switches(ctx))
      .unwrap_or_default()
  }

  fn get_drivers<'a>(&self, ctx: &'a Context) -> Vec<&'a Driver> {
    self
      .as_ref()
      .map(|sel| sel.get_drivers(ctx))
      .unwrap_or_default()
  }

  fn get_illuminations<'a>(&self, ctx: &'a Context) -> Vec<&'a AddressableIllumination> {
    self
      .as_ref()
      .map(|sel| sel.get_illuminations(ctx))
      .unwrap_or_default()
  }
}

#[cfg(test)]
mod tests {
  use crate::NativeIdentity;
  use crate::tags::Playfield;

  use super::*;

  #[test]
  fn name_selection() {
    let selection = HardwareSelection::name("switch1");

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![],
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn group_selection() {
    let selection = HardwareSelection::group(vec!["switch1", "switch2"]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![],
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn tag_selection() {
    let selection = HardwareSelection::tag::<Playfield>();

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![Box::new(Playfield)],
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn and_selection() {
    let selection = HardwareSelection::and(
      HardwareSelection::name("switch1"),
      HardwareSelection::tag::<Playfield>(),
    );

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![Box::new(Playfield)],
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn or_selection() {
    let selection = HardwareSelection::or(
      HardwareSelection::name("switch1"),
      HardwareSelection::name("switch2"),
    );

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![],
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn any_of_selection() {
    let selection = HardwareSelection::any_of(vec![
      HardwareSelection::name("switch1"),
      HardwareSelection::name("switch2"),
    ]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![],
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn all_of_selection() {
    let selection = HardwareSelection::all_of(vec![
      HardwareSelection::name("switch1"),
      HardwareSelection::tag::<Playfield>(),
    ]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![Box::new(Playfield)],
    };

    assert!(selection.matches_switch(&switch));
  }
}
