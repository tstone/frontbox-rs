use itertools::Itertools;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

/// Every light on the machine by name, and the planes they're placed on, for finding them by [`LightQ`]
#[derive(Debug, Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LightLookup {
  by_name: HashMap<String, Light>,
  configs: HashMap<String, LedConfiguration>,
  /// Keyed the way the hardware sets LEDs: (board and breakout, index)
  #[serde(skip)] // redundant with by_name for web console
  order_by_index: HashMap<(ExpAddress, u8), ChannelOrder>,
  /// What the app was given, for finding one by name
  #[serde(skip)]
  planes: Vec<&'static ReferencePlane>,
}

impl LightLookup {
  pub fn new(boards: &Vec<ResolvedExpansionBoard>) -> Self {
    let mut by_name = HashMap::new();
    let mut configs = HashMap::new();
    let mut order_by_index = HashMap::new();

    for board in boards {
      for (port_index, port) in board.led_ports.iter().enumerate() {
        for led in &port.leds {
          let address = LedAddress::new(led.assignment, port_index as u8, led.id as u8);

          let order = led
            .definition
            .config
            .as_ref()
            .map(|config| config.channel_order)
            .unwrap_or_default();
          order_by_index.insert((address.exp, address.index), order);

          by_name.insert(
            led.definition.name.to_string(),
            Light {
              name: led.definition.name.to_string(),
              address: LightAddress::rgb(address),
              tags: led.definition.tags.clone(),
              location: led.definition.location,
            },
          );

          led
            .definition
            .config
            .as_ref()
            .and_then(|config| configs.insert(led.definition.name.to_string(), config.clone()));
        }
      }
    }

    Self {
      by_name,
      configs,
      order_by_index,
      planes: Vec::new(),
    }
  }

  pub fn by_name(&self, name: &str) -> Option<&Light> {
    self.by_name.get(name)
  }

  pub fn by_tag<T: Tag + 'static>(&self) -> Vec<&Light> {
    self
      .by_name
      .values()
      .filter(|light| {
        light
          .tags
          .iter()
          .any(|tag| <dyn Tag>::as_any(&**tag).is::<T>())
      })
      .collect()
  }

  pub fn add_plane(&mut self, plane: &'static ReferencePlane) {
    self.planes.push(plane);
  }

  /// A plane added to the app, by its name
  pub fn plane(&self, name: &str) -> Option<&'static ReferencePlane> {
    self.planes.iter().find(|plane| plane.name == name).copied()
  }

  fn matches(&self, query: &LightQ, light: &Light) -> bool {
    match query {
      LightQ::Every => true,
      LightQ::Name(name) => light.name == *name,
      LightQ::Names(names) => names.contains(&light.name),
      LightQ::Tag(tag) => light.has_tag_named(tag),
      LightQ::And(qs) => qs.iter().all(|q| self.matches(q, light)),
      LightQ::Or(qs) => qs.iter().any(|q| self.matches(q, light)),
      LightQ::Location(plane, region) => self.plane(plane).zip(light.location).is_some_and(|(plane, location)| region.within(plane.to_relative(location))),
      LightQ::Reverse(other) => self.matches(other, light),
      LightQ::Skip(other, _) => self.query_iter(other).contains(light),
      LightQ::Take(other, _) => self.query_iter(other).contains(light),
    }
  }

  pub fn query_iter<'a>(&'a self, query: &'a LightQ) -> Box<dyn Iterator<Item = &'a Light> + 'a> {
    match query {
      LightQ::Name(name) => Box::new(self.by_name.get(name).into_iter()),
      LightQ::Names(names) => Box::new(names.iter().filter_map(|n| self.by_name.get(n))),
      // A light several of them reach comes once, at the first of them
      LightQ::Or(queries) => Box::new(
        queries
          .iter()
          .flat_map(|q| self.query_iter(q))
          .unique_by(|light| light.name.as_str()),
      ),
      LightQ::Skip(other, n) => Box::new(self.query_iter(other).skip(*n)),
      LightQ::Take(other, n) => Box::new(self.query_iter(other).take(*n)),
      LightQ::Reverse(other) => {
        let mut items: Vec<_> = self.query_iter(other).collect();
        items.reverse();
        Box::new(items.into_iter())
      }
      _ => {
        let mut matches: Vec<&Light> = self
          .by_name
          .values()
          .filter_map(|light| {
            if self.matches(query, light) {
              Some(light)
            } else {
              None
            }
          })
          .collect();

        // For other non-ordered results (e.g. by take) sort by hardware address to maintain consistent order
        matches.sort_by_key(|light| light.address.hardware_order());
        Box::new(matches.into_iter())
      }
    }
  }

  pub fn config(&self, name: &'static str) -> Option<&LedConfiguration> {
    self
      .by_name
      .get(name)
      .and_then(|light| self.configs.get(&light.name))
  }

  /// The channel order of the LED at `index` on a board's breakout, or the default for an LED that isn't defined
  pub(crate) fn order_at(&self, address: ExpAddress, index: u8) -> ChannelOrder {
    self
      .order_by_index
      .get(&(address, index))
      .copied()
      .unwrap_or_default()
  }

  /// The index and channel order of every LED defined on a board's breakout
  pub(crate) fn orders_on(&self, address: ExpAddress) -> impl Iterator<Item = (u8, ChannelOrder)> + '_ {
    self
      .order_by_index
      .iter()
      .filter(move |((exp, _), _)| *exp == address)
      .map(|((_, index), order)| (*index, *order))
  }
}

impl Deref for LightLookup {
  type Target = HashMap<String, Light>;

  fn deref(&self) -> &Self::Target {
    &self.by_name
  }
}

impl DerefMut for LightLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.by_name
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(frontbox_derive::Tag)]
  struct Insert;
  #[derive(frontbox_derive::Tag)]
  struct Lane;

  /// Lights named by their (board, breakout, index)
  fn lookup(addresses: &[(u8, Option<u8>, u8)]) -> LightLookup {
    let mut lights = LightLookup::default();
    for &(board, breakout, index) in addresses {
      let name = format!("{board}.{}.{index}", breakout.unwrap_or(0));
      let address = LightAddress::rgb(LedAddress::new(ExpAddress::new(board, breakout), 0, index));
      lights.insert(name.clone(), Light { name, address, ..Default::default() });
    }
    lights
  }

  fn names<'a>(lights: &'a LightLookup, query: &'a LightQ) -> Vec<&'a str> {
    lights.query_iter(query).map(|light| light.name.as_str()).collect()
  }

  #[test]
  fn lights_come_in_hardware_order() {
    let lights = lookup(&[(2, Some(0), 0), (1, Some(1), 1), (1, Some(1), 0), (1, Some(0), 5)]);
    assert_eq!(names(&lights, &LightQ::every()), ["1.0.5", "1.1.0", "1.1.1", "2.0.0"]);
  }

  #[test]
  fn hardware_order_holds_on_board_zero() {
    let lights = lookup(&[(0, None, 2), (0, None, 0), (0, None, 1)]);
    assert_eq!(names(&lights, &LightQ::every()), ["0.0.0", "0.0.1", "0.0.2"]);
  }

  #[test]
  fn a_tag_comes_in_hardware_order() {
    let mut lights = lookup(&[(2, Some(0), 0), (1, Some(1), 0), (1, Some(0), 5)]);
    for name in ["2.0.0", "1.0.5"] {
      lights.get_mut(name).unwrap().tags.push(Box::new(Insert));
    }
    lights.get_mut("1.1.0").unwrap().tags.push(Box::new(Lane));
    assert_eq!(names(&lights, &LightQ::Tag("Insert".into())), ["1.0.5", "2.0.0"]);
  }

  #[test]
  fn or_runs_a_query_at_a_time_and_reaches_a_light_once() {
    let mut lights = lookup(&[(2, Some(0), 0), (1, Some(1), 0), (1, Some(0), 5)]);
    for name in ["2.0.0", "1.1.0"] {
      lights.get_mut(name).unwrap().tags.push(Box::new(Lane));
    }
    for name in ["1.0.5", "1.1.0"] {
      lights.get_mut(name).unwrap().tags.push(Box::new(Insert));
    }
    let lane_first = LightQ::Or(vec![LightQ::Tag("Lane".into()), LightQ::Tag("Insert".into())]);
    assert_eq!(names(&lights, &lane_first), ["1.1.0", "2.0.0", "1.0.5"]);
  }

  #[test]
  fn names_come_in_the_order_given() {
    let lights = lookup(&[(1, Some(0), 0), (1, Some(0), 1), (1, Some(0), 2)]);
    let query = LightQ::names(["1.0.2", "1.0.0", "1.0.1"]);
    assert_eq!(names(&lights, &query), ["1.0.2", "1.0.0", "1.0.1"]);
  }

  #[test]
  fn a_tag_no_light_has_reaches_none() {
    let lights = lookup(&[(1, Some(0), 0)]);
    assert!(names(&lights, &LightQ::Tag("Insert".into())).is_empty());
  }

  #[test]
  fn reverse_is_hardware_order_backwards() {
    let lights = lookup(&[(1, Some(0), 5), (1, Some(1), 0), (2, Some(0), 0)]);
    assert_eq!(names(&lights, &LightQ::every().reverse()), ["2.0.0", "1.1.0", "1.0.5"]);
  }
}
