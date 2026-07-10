use std::any::TypeId;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareQuery {
  Name(&'static str),
  Tag(TypeId),
  And(Box<HardwareQuery>, Box<HardwareQuery>),
  Or(Box<HardwareQuery>, Box<HardwareQuery>),
}

impl HardwareQuery {
  pub fn matches_switch(&self, switch: &Switch) -> bool {
    match self {
      Self::Name(name) => switch.name == *name,
      Self::Tag(tag) => switch.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_switch(switch) && right.matches_switch(switch),
      Self::Or(left, right) => left.matches_switch(switch) || right.matches_switch(switch),
    }
  }

  pub fn matches_driver(&self, driver: &Driver) -> bool {
    match self {
      Self::Name(name) => driver.name == *name,
      Self::Tag(tag) => driver.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_driver(driver) && right.matches_driver(driver),
      Self::Or(left, right) => left.matches_driver(driver) || right.matches_driver(driver),
    }
  }

  pub fn matches_illumination(&self, illumination: &AddressableIllumination) -> bool {
    match self {
      Self::Name(name) => illumination.name() == *name,
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
    ctx.switches.query(&self)
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

  pub fn get_leds<'a>(&self, ctx: &'a Context) -> Vec<&'a AddressableLed> {
    self
      .get_illuminations(ctx)
      .iter()
      .flat_map(|illum| &illum.leds)
      .collect()
  }
}

pub trait HardwareTagExt {
  fn get_switches<'a>(&self, ctx: &'a Context) -> Vec<&'a Switch>;
  fn get_drivers<'a>(&self, ctx: &'a Context) -> Vec<&'a Driver>;
  fn get_illuminations<'a>(&self, ctx: &'a Context) -> Vec<&'a AddressableIllumination>;
}

impl HardwareTagExt for Option<HardwareQuery> {
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
    let selection = Q::name("switch1");

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
    let selection = Q::tag::<Playfield>();

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
    let selection = Q::and(Q::name("switch1"), Q::tag::<Playfield>());

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
    let selection = Q::or(Q::name("switch1"), Q::name("switch2"));

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
    let selection = Q::any_of(vec![Q::name("switch1"), Q::name("switch2")]);

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
    let selection = Q::all_of(vec![Q::name("switch1"), Q::tag::<Playfield>()]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      native: NativeIdentity::new(0, 1),
      tags: vec![Box::new(Playfield)],
    };

    assert!(selection.matches_switch(&switch));
  }
}
