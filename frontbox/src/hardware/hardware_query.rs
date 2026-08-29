use std::any::TypeId;

use indexmap::IndexSet;

use crate::prelude::*;

pub enum QueryableHardware {
  Switch,
  Driver,
  LED,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HardwareQuery {
  Name(String),
  Names(IndexSet<String>),
  Tag(TypeId),
  And(Vec<HardwareQuery>),
  Or(Vec<HardwareQuery>),
  Location(ReferencePlane, Region),
}

impl HardwareQuery {
  pub fn or(self, other: Self) -> Self {
    Self::Or(vec![self, other])
  }

  pub fn and(self, other: Self) -> Self {
    Self::And(vec![self, other])
  }

  /// As queries get more complex, it can sometimes be useful to pre-compute them into a list of names rather than dynamically re-computing them each time they are needed.
  pub fn precompute(&self, hardware: QueryableHardware, ctx: &ServiceContext) -> Self {
    match hardware {
      QueryableHardware::Switch => HardwareQuery::Names(
        self
          .get_switch_names(ctx)
          .into_iter()
          .map(Into::into)
          .collect(),
      ),
      QueryableHardware::Driver => HardwareQuery::Names(
        self
          .get_driver_names(ctx)
          .into_iter()
          .map(Into::into)
          .collect(),
      ),
      QueryableHardware::LED => HardwareQuery::Names(
        self
          .get_led_names(ctx)
          .into_iter()
          .map(Into::into)
          .collect(),
      ),
    }
  }

  pub fn matches_switch(&self, switch: &Switch) -> bool {
    match self {
      Self::Name(name) => switch.name == *name,
      Self::Names(names) => names.contains(switch.name),
      Self::Tag(tag) => switch.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_switch(switch)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_switch(switch)),
      Self::Location(plane, region) => switch
        .location
        .map(|location| region.within(plane.to_relative(location)))
        .unwrap_or(false),
    }
  }

  pub fn matches_driver(&self, driver: &Driver) -> bool {
    match self {
      Self::Name(name) => driver.name == *name,
      Self::Names(names) => names.contains(driver.name),
      Self::Tag(tag) => driver.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_driver(driver)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_driver(driver)),
      Self::Location(plane, region) => driver
        .location
        .map(|location| region.within(plane.to_relative(location)))
        .unwrap_or(false),
    }
  }

  pub fn matches_led(&self, led: &LED) -> bool {
    match self {
      Self::Name(name) => led.name == *name,
      Self::Names(names) => names.contains(&led.name),
      Self::Tag(tag) => led.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_led(led)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_led(led)),
      Self::Location(plane, region) => led
        .location
        .map(|location| region.within(plane.to_relative(location)))
        .unwrap_or(false),
    }
  }

  /// Resolve the query into a reference for all matching Switches
  pub fn get_switches<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c Switch> {
    ctx.switches.query(&self)
  }

  /// Resolve the query into a the names of all matching Switches
  pub fn get_switch_names<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'static str> {
    ctx.switches.query(&self).iter().map(|sw| sw.name).collect()
  }

  /// Resolve the query into a reference for all matching Drivers
  pub fn get_drivers<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c Driver> {
    ctx.drivers.by_selection(&self)
  }

  /// Resolve the query into a the names of all matching Drivers
  pub fn get_driver_names<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'static str> {
    ctx
      .drivers
      .by_selection(&self)
      .iter()
      .map(|d| d.name)
      .collect()
  }

  /// Resolve the query into a reference for all matching LEDs
  pub fn get_leds<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c LED> {
    let leds = ctx
      .leds
      .values()
      .filter(|led| self.matches_led(led))
      .collect::<Vec<_>>();

    log::trace!(target: "frontbox::query", "HardwareQuery: Got {} LEDS for query {:?}: {:?}", leds.len(), self, leds);
    leds
  }

  /// Resolve the query into a the names of all matching LEDs
  pub fn get_led_names<'c>(&self, ctx: &'c ServiceContext) -> Vec<String> {
    ctx
      .leds
      .query(&self)
      .iter()
      .map(|d| d.name.clone())
      .collect()
  }

  /// Resolve the query into a the address of all matching LEDs
  pub fn get_leds_addresses(&self, ctx: &ServiceContext) -> Vec<LedAddress> {
    let addrs = match self {
      // maintain order for name/names
      Self::Name(name) => match ctx.leds.get(name) {
        Some(led) => vec![led.address.clone()],
        None => Vec::new(),
      },
      Self::Names(names) => names
        .iter()
        .filter_map(|n| ctx.leds.get(n))
        .map(|led| led.address.clone())
        .collect(),
      Self::Or(qs) => qs.iter().flat_map(|q| q.get_leds_addresses(ctx)).collect(),
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

        // For other non-ordered results (e.g. by take) sort by hardware addres to maintain consistent order
        matches.sort_by_key(|addr| {
          ((addr.exp.board_address as u32 * 10) + addr.exp.breakout.unwrap_or(0) as u32)
            * addr.index as u32
        });
        matches
      }
    };

    log::trace!(target: "frontbox::query", "HardwareQuery: Got {} LEDS for query {:?}: {:?}", addrs.len(), self, addrs);
    addrs
  }
}

pub trait HardwareTagExt {
  fn get_switches<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a Switch>;
  fn get_drivers<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a Driver>;
  fn get_leds<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a LED>;
}

impl HardwareTagExt for Option<HardwareQuery> {
  fn get_switches<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c Switch> {
    self
      .as_ref()
      .map(|q| q.get_switches(ctx))
      .unwrap_or_default()
  }

  fn get_drivers<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c Driver> {
    self
      .as_ref()
      .map(|q| q.get_drivers(ctx))
      .unwrap_or_default()
  }

  fn get_leds<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c LED> {
    self.as_ref().map(|q| q.get_leds(ctx)).unwrap_or_default()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tags::Playfield;

  #[test]
  fn name_selection() {
    let q = Q::name("switch1");

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

    assert!(q.matches_switch(&switch));
  }

  #[test]
  fn tag_selection() {
    let q = Q::tag::<Playfield>();

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

    assert!(q.matches_switch(&switch));
  }

  #[test]
  fn and_selection() {
    let q = Q::and(Q::name("switch1"), Q::tag::<Playfield>());

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

    assert!(q.matches_switch(&switch));
  }

  #[test]
  fn or_selection() {
    let q = Q::or(Q::name("switch1"), Q::name("switch2"));

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

    assert!(q.matches_switch(&switch));
  }

  #[test]
  fn any_of_selection() {
    let q = Q::any(vec![&Q::name("switch1"), &Q::name("switch2")]);

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

    assert!(q.matches_switch(&switch));
  }

  #[test]
  fn all_of_selection() {
    let q = Q::all(vec![&Q::name("switch1"), &Q::tag::<Playfield>()]);

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

    assert!(q.matches_switch(&switch));
  }

  #[test]
  fn get_leds_addresses_name() {
    let mut context = TestContext::default();
    context.base.leds.insert(
      "led1".to_string(),
      LED {
        name: "led1".to_string(),
        address: LedAddress::new(ExpAddress::default(), 1),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led2".to_string(),
      LED {
        name: "led2".to_string(),
        address: LedAddress::new(ExpAddress::default(), 2),
        ..Default::default()
      },
    );

    let q = Q::name("led1");
    let leds = q.get_leds_addresses(&context.svc_ctx());

    assert_eq!(leds.len(), 1);
    assert_eq!(leds[0].index, 1);
  }

  #[test]
  fn get_leds_addresses_names() {
    let mut context = TestContext::default();
    context.base.leds.insert(
      "led1".to_string(),
      LED {
        name: "led1".to_string(),
        address: LedAddress::new(ExpAddress::default(), 1),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led2".to_string(),
      LED {
        name: "led2".to_string(),
        address: LedAddress::new(ExpAddress::default(), 2),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led3".to_string(),
      LED {
        name: "led3".to_string(),
        address: LedAddress::new(ExpAddress::default(), 3),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led4".to_string(),
      LED {
        name: "led4".to_string(),
        address: LedAddress::new(ExpAddress::default(), 4),
        ..Default::default()
      },
    );

    let q = Q::names(vec!["led2", "led3"]);
    let leds = q.get_leds_addresses(&context.svc_ctx());

    assert_eq!(leds.len(), 2);
    assert_eq!(leds[0].index, 2);
    assert_eq!(leds[1].index, 3);
  }

  // The goal here is to ensure that using names, even through `any` maintain their order
  #[test]
  fn get_leds_addresses_any_names() {
    let mut context = TestContext::default();
    context.base.leds.insert(
      "led1".to_string(),
      LED {
        name: "led1".to_string(),
        address: LedAddress::new(ExpAddress::default(), 5),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led2".to_string(),
      LED {
        name: "led2".to_string(),
        address: LedAddress::new(ExpAddress::default(), 8),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led3".to_string(),
      LED {
        name: "led3".to_string(),
        address: LedAddress::new(ExpAddress::default(), 3),
        ..Default::default()
      },
    );
    context.base.leds.insert(
      "led4".to_string(),
      LED {
        name: "led4".to_string(),
        address: LedAddress::new(ExpAddress::default(), 4),
        ..Default::default()
      },
    );

    let q = Q::any(vec![
      &Q::names(vec!["led1", "led2"]),
      &Q::names(vec!["led3", "led4"]),
    ]);
    let leds = q.get_leds_addresses(&context.svc_ctx());

    assert_eq!(leds[0].index, 5);
    assert_eq!(leds[1].index, 8);
    assert_eq!(leds[2].index, 3);
    assert_eq!(leds[3].index, 4);
  }
}
