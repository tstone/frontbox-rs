use std::any::TypeId;

use indexmap::IndexSet;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareQuery {
  Name(String),
  Names(IndexSet<String>),
  Tag(TypeId),
  And(Box<HardwareQuery>, Box<HardwareQuery>),
  Or(Box<HardwareQuery>, Box<HardwareQuery>),
  Reverse(Box<HardwareQuery>), // TODO: change this to Order w/ the serpentine methods
}

impl HardwareQuery {
  pub fn reverse(self) -> HardwareQuery {
    HardwareQuery::Reverse(Box::new(self))
  }

  pub fn matches_switch(&self, switch: &Switch) -> bool {
    match self {
      Self::Name(name) => switch.name == *name,
      Self::Names(names) => names.contains(switch.name),
      Self::Tag(tag) => switch.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_switch(switch) && right.matches_switch(switch),
      Self::Or(left, right) => left.matches_switch(switch) || right.matches_switch(switch),
      Self::Reverse(q) => q.matches_switch(switch),
    }
  }

  pub fn matches_driver(&self, driver: &Driver) -> bool {
    match self {
      Self::Name(name) => driver.name == *name,
      Self::Names(names) => names.contains(driver.name),
      Self::Tag(tag) => driver.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_driver(driver) && right.matches_driver(driver),
      Self::Or(left, right) => left.matches_driver(driver) || right.matches_driver(driver),
      Self::Reverse(q) => q.matches_driver(driver),
    }
  }

  pub fn matches_led(&self, led: &LED) -> bool {
    match self {
      Self::Name(name) => led.name == *name,
      Self::Names(names) => names.contains(&led.name),
      Self::Tag(tag) => led.has_typed_tag(*tag),
      Self::And(left, right) => left.matches_led(led) && right.matches_led(led),
      Self::Or(left, right) => left.matches_led(led) || right.matches_led(led),
      Self::Reverse(q) => q.matches_led(led),
    }
  }

  pub fn get_switches<'c>(&self, ctx: &'c Context) -> Vec<&'c Switch> {
    ctx.switches.query(&self)
  }

  pub fn get_drivers<'c>(&self, ctx: &'c Context) -> Vec<&'c Driver> {
    ctx.drivers.by_selection(&self)
  }

  pub fn get_leds<'c>(&self, ctx: &'c Context) -> Vec<&'c LED> {
    ctx
      .leds
      .values()
      .filter(|led| self.matches_led(led))
      .collect()
  }

  pub fn get_leds_addresses(&self, ctx: &Context) -> Vec<LedAddress> {
    ctx
      .leds
      .values()
      .filter_map(|led| {
        if self.matches_led(led) {
          Some(led.address.clone())
        } else {
          None
        }
      })
      .collect()
  }
}

pub trait HardwareTagExt {
  fn get_switches<'a>(&self, ctx: &'a Context) -> Vec<&'a Switch>;
  fn get_drivers<'a>(&self, ctx: &'a Context) -> Vec<&'a Driver>;
  fn get_leds<'a>(&self, ctx: &'a Context) -> Vec<&'a LED>;
}

impl HardwareTagExt for Option<HardwareQuery> {
  fn get_switches<'c>(&self, ctx: &'c Context) -> Vec<&'c Switch> {
    self
      .as_ref()
      .map(|q| q.get_switches(ctx))
      .unwrap_or_default()
  }

  fn get_drivers<'c>(&self, ctx: &'c Context) -> Vec<&'c Driver> {
    self
      .as_ref()
      .map(|q| q.get_drivers(ctx))
      .unwrap_or_default()
  }

  fn get_leds<'c>(&self, ctx: &'c Context) -> Vec<&'c LED> {
    self.as_ref().map(|q| q.get_leds(ctx)).unwrap_or_default()
  }
}

#[cfg(test)]
mod tests {
  use crate::tags::Playfield;

  use super::*;

  #[test]
  fn name_selection() {
    let selection = Q::name("switch1");

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![],
      location: None,
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn tag_selection() {
    let selection = Q::tag::<Playfield>();

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn and_selection() {
    let selection = Q::and(Q::name("switch1"), Q::tag::<Playfield>());

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn or_selection() {
    let selection = Q::or(Q::name("switch1"), Q::name("switch2"));

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn any_of_selection() {
    let selection = Q::any_of(vec![Q::name("switch1"), Q::name("switch2")]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(selection.matches_switch(&switch));
  }

  #[test]
  fn all_of_selection() {
    let selection = Q::all_of(vec![Q::name("switch1"), Q::tag::<Playfield>()]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(selection.matches_switch(&switch));
  }
}
