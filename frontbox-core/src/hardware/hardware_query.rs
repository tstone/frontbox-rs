use std::any::TypeId;

use indexmap::IndexSet;

use crate::prelude::*;

pub enum QueryableHardware {
  Switch,
  Driver,
  Light,
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
      QueryableHardware::Light => HardwareQuery::Names(
        self
          .get_light_names(ctx)
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

  pub fn matches_light(&self, light: &Light) -> bool {
    match self {
      Self::Name(name) => light.name == *name,
      Self::Names(names) => names.contains(&light.name),
      Self::Tag(tag) => light.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches_light(light)),
      Self::Or(qs) => qs.iter().any(|q| q.matches_light(light)),
      Self::Location(plane, region) => light
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

  /// Resolve the query into a reference for all matching lights
  pub fn get_lights<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c Light> {
    let lights = ctx
      .lights
      .values()
      .filter(|light| self.matches_light(light))
      .collect::<Vec<_>>();

    log::trace!(target: "frontbox::hardware", "HardwareQuery: Got {} lights for query {:?}: {:?}", lights.len(), self, lights);
    lights
  }

  /// Resolve the query into a the names of all matching lights
  pub fn get_light_names<'c>(&self, ctx: &'c ServiceContext) -> Vec<String> {
    ctx
      .lights
      .query(&self)
      .iter()
      .map(|d| d.name.clone())
      .collect()
  }

  /// Resolve the query into a the address of all matching lights
  pub fn get_light_addresses(&self, ctx: &ServiceContext) -> Vec<LightAddress> {
    let addrs = match self {
      // maintain order for name/names
      Self::Name(name) => match ctx.lights.get(name) {
        Some(light) => vec![light.address.clone()],
        None => Vec::new(),
      },
      Self::Names(names) => names
        .iter()
        .filter_map(|n| ctx.lights.get(n))
        .map(|light| light.address.clone())
        .collect(),
      Self::Or(qs) => qs.iter().flat_map(|q| q.get_light_addresses(ctx)).collect(),
      _ => {
        let mut matches: Vec<LightAddress> = ctx
          .lights
          .values()
          .filter_map(|light| {
            if self.matches_light(light) {
              Some(light.address.clone())
            } else {
              None
            }
          })
          .collect();

        // For other non-ordered results (e.g. by take) sort by hardware addres to maintain consistent order
        matches.sort_by_key(|addr| addr.hardware_order());
        matches
      }
    };

    log::trace!(target: "frontbox::hardware", "HardwareQuery: Got {} lights for query {:?}: {:?}", addrs.len(), self, addrs);
    addrs
  }
}

pub trait HardwareTagExt {
  fn get_switches<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a Switch>;
  fn get_drivers<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a Driver>;
  fn get_lights<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a Light>;
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

  fn get_lights<'c>(&self, ctx: &'c ServiceContext) -> Vec<&'c Light> {
    self.as_ref().map(|q| q.get_lights(ctx)).unwrap_or_default()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[derive(frontbox_derive::Tag)]
  struct Playfield;

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
  fn get_light_addresses_name() {
    let mut context = TestContext::default();
    context.base.lights.insert(
      "light1".to_string(),
      Light {
        name: "light1".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 1)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light2".to_string(),
      Light {
        name: "light2".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 2)),
        ..Default::default()
      },
    );

    let q = Q::name("light1");
    let lights = q.get_light_addresses(&context.svc_ctx());

    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].led().unwrap().index, 1);
  }

  #[test]
  fn get_light_addresses_names() {
    let mut context = TestContext::default();
    context.base.lights.insert(
      "light1".to_string(),
      Light {
        name: "light1".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 1)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light2".to_string(),
      Light {
        name: "light2".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 2)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light3".to_string(),
      Light {
        name: "light3".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 3)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light4".to_string(),
      Light {
        name: "light4".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 4)),
        ..Default::default()
      },
    );

    let q = Q::names(vec!["light2", "light3"]);
    let lights = q.get_light_addresses(&context.svc_ctx());

    assert_eq!(lights.len(), 2);
    assert_eq!(lights[0].led().unwrap().index, 2);
    assert_eq!(lights[1].led().unwrap().index, 3);
  }

  // The goal here is to ensure that using names, even through `any` maintain their order
  #[test]
  fn get_light_addresses_any_names() {
    let mut context = TestContext::default();
    context.base.lights.insert(
      "light1".to_string(),
      Light {
        name: "light1".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 5)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light2".to_string(),
      Light {
        name: "light2".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 8)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light3".to_string(),
      Light {
        name: "light3".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 3)),
        ..Default::default()
      },
    );
    context.base.lights.insert(
      "light4".to_string(),
      Light {
        name: "light4".to_string(),
        address: LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, 4)),
        ..Default::default()
      },
    );

    let q = Q::any(vec![
      &Q::names(vec!["light1", "light2"]),
      &Q::names(vec!["light3", "light4"]),
    ]);
    let lights = q.get_light_addresses(&context.svc_ctx());

    assert_eq!(lights[0].led().unwrap().index, 5);
    assert_eq!(lights[1].led().unwrap().index, 8);
    assert_eq!(lights[2].led().unwrap().index, 3);
    assert_eq!(lights[3].led().unwrap().index, 4);
  }
}
