use std::any::TypeId;

use indexmap::IndexSet;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareQuery {
  Name(String),
  Names(IndexSet<String>),
  Tag(TypeId),
  And(Vec<HardwareQuery>),
  Or(Vec<HardwareQuery>),
  Reverse(Box<HardwareQuery>), // TODO: change this to Order w/ the serpentine methods
}

impl HardwareQuery {
  pub fn reverse(self) -> HardwareQuery {
    HardwareQuery::Reverse(Box::new(self))
  }

  pub fn or(self, other: Self) -> Self {
    Self::Or(vec![self, other])
  }

  pub fn and(self, other: Self) -> Self {
    Self::And(vec![self, other])
  }

  pub fn matches_switch(&self, switch: &Switch) -> bool {
    match self {
      Self::Name(name) => switch.name == *name,
      Self::Names(names) => names.contains(switch.name),
      Self::Tag(tag) => switch.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_switch(switch)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_switch(switch)),
      Self::Reverse(q) => q.matches_switch(switch),
    }
  }

  pub fn matches_driver(&self, driver: &Driver) -> bool {
    match self {
      Self::Name(name) => driver.name == *name,
      Self::Names(names) => names.contains(driver.name),
      Self::Tag(tag) => driver.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_driver(driver)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_driver(driver)),
      Self::Reverse(q) => q.matches_driver(driver),
    }
  }

  pub fn matches_led(&self, led: &LED) -> bool {
    match self {
      Self::Name(name) => led.name == *name,
      Self::Names(names) => names.contains(&led.name),
      Self::Tag(tag) => led.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_led(led)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_led(led)),
      Self::Reverse(q) => q.matches_led(led),
    }
  }

  /// Resolve the query into a reference for all matching Switches
  pub fn get_switches<'c>(&self, ctx: &'c SystemContext) -> Vec<&'c Switch> {
    ctx.switches.query(&self)
  }

  /// Resolve the query into a the names of all matching Switches
  pub fn get_switch_names<'c>(&self, ctx: &'c SystemContext) -> Vec<&'static str> {
    ctx.switches.query(&self).iter().map(|sw| sw.name).collect()
  }

  /// Resolve the query into a reference for all matching Drivers
  pub fn get_drivers<'c>(&self, ctx: &'c SystemContext) -> Vec<&'c Driver> {
    ctx.drivers.by_selection(&self)
  }

  /// Resolve the query into a the names of all matching Drivers
  pub fn get_driver_names<'c>(&self, ctx: &'c SystemContext) -> Vec<&'static str> {
    ctx
      .drivers
      .by_selection(&self)
      .iter()
      .map(|d| d.name)
      .collect()
  }

  /// Resolve the query into a reference for all matching LEDs
  pub fn get_leds<'c>(&self, ctx: &'c SystemContext) -> Vec<&'c LED> {
    let leds = ctx
      .leds
      .values()
      .filter(|led| self.matches_led(led))
      .collect::<Vec<_>>();

    log::trace!(target: "frontbox::query", "HardwareQuery: Got {} LEDS for query {:?}: {:?}", leds.len(), self, leds);
    leds
  }

  /// Resolve the query into a the address of all matching LEDs
  pub fn get_leds_addresses(&self, ctx: &SystemContext) -> Vec<LedAddress> {
    let addrs = match self {
      Self::Name(name) => vec![ctx.leds.get(name).unwrap().address.clone()],
      Self::Names(names) => names
        .iter()
        .map(|n| ctx.leds.get(n).unwrap().address.clone())
        .collect(),
      _ => {
        let mut matches: Vec<LedAddress> = ctx
          .leds
          .values()
          .filter_map(|led| {
            if self.matches_led(led) {
              Some(led.address.clone())
            } else {
              None
            }
          })
          .collect();
        // maintain consistent order
        matches.sort_by_key(|addr| addr.index);
        matches
      }
    };

    log::trace!(target: "frontbox::query", "HardwareQuery: Got {} LEDS for query {:?}: {:?}", addrs.len(), self, addrs);
    addrs
  }
}

pub trait HardwareTagExt {
  fn get_switches<'a>(&self, ctx: &'a SystemContext) -> Vec<&'a Switch>;
  fn get_drivers<'a>(&self, ctx: &'a SystemContext) -> Vec<&'a Driver>;
  fn get_leds<'a>(&self, ctx: &'a SystemContext) -> Vec<&'a LED>;
}

impl HardwareTagExt for Option<HardwareQuery> {
  fn get_switches<'c>(&self, ctx: &'c SystemContext) -> Vec<&'c Switch> {
    self
      .as_ref()
      .map(|q| q.get_switches(ctx))
      .unwrap_or_default()
  }

  fn get_drivers<'c>(&self, ctx: &'c SystemContext) -> Vec<&'c Driver> {
    self
      .as_ref()
      .map(|q| q.get_drivers(ctx))
      .unwrap_or_default()
  }

  fn get_leds<'c>(&self, ctx: &'c SystemContext) -> Vec<&'c LED> {
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
    let selection = Q::any_of(vec![&Q::name("switch1"), &Q::name("switch2")]);

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
    let selection = Q::all_of(vec![&Q::name("switch1"), &Q::tag::<Playfield>()]);

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
