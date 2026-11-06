use itertools::Itertools;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

#[derive(Debug, Clone, Default, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct LedLookup {
  by_name: HashMap<String, LED>,
  configs: HashMap<String, LedConfiguration>,
  /// Keyed the way the hardware sets LEDs: (board address, breakout, index)
  #[serde(skip)] // redundant with by_name for web console
  channels_by_index: HashMap<(u8, Option<u8>, u8), LedChannels>,
}

impl LedLookup {
  pub fn new(boards: &Vec<ResolvedExpansionBoard>) -> Self {
    let mut by_name = HashMap::new();
    let mut configs = HashMap::new();
    let mut channels_by_index = HashMap::new();

    for board in boards {
      for port in &board.led_ports {
        for led in &port.leds {
          let address = LedAddress {
            exp: led.assignment.clone(),
            index: led.id as u8,
          };

          let channels = led
            .definition
            .config
            .as_ref()
            .map(|config| config.channels)
            .unwrap_or_default();
          channels_by_index.insert(
            (address.board(), address.breakout(), address.index),
            channels,
          );

          by_name.insert(
            led.definition.name.to_string(),
            LED {
              name: led.definition.name.to_string(),
              address,
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
      channels_by_index,
    }
  }

  pub fn by_name(&self, name: &str) -> Option<&LED> {
    self.by_name.get(name)
  }

  pub fn by_tag<T: Tag + 'static>(&self) -> Vec<&LED> {
    self
      .by_name
      .values()
      .filter(|led| {
        led
          .tags
          .iter()
          .any(|tag| <dyn Tag>::as_any(&**tag).is::<T>())
      })
      .collect()
  }

  /// The query for a tag known only by its name, as it is from outside the app. One that no LED has reaches none.
  pub fn tag_named(&self, name: &str) -> LedQ {
    self
      .by_name
      .values()
      .flat_map(|led| &led.tags)
      .find(|tag| tag.tag_name() == name)
      .map_or_else(
        || LedQ::names(Vec::<String>::new()),
        |tag| LedQ::Tag(<dyn Tag>::as_any(&**tag).type_id()),
      )
  }

  fn matches(&self, query: &LedQ, led: &LED) -> bool {
    match query {
      LedQ::Every => true,
      LedQ::Name(name) => led.name == *name,
      LedQ::Names(names) => names.contains(&led.name),
      LedQ::Tag(tag) => led.has_typed_tag(*tag),
      LedQ::And(qs) => qs.iter().all(|q| self.matches(q, led)),
      LedQ::Or(qs) => qs.iter().any(|q| self.matches(q, led)),
      LedQ::Location(plane, region) => led
        .location
        .map(|location| region.within(plane.to_relative(location)))
        .unwrap_or(false),
      LedQ::Reverse(other) => self.matches(other, led),
      LedQ::Skip(other, _) => self.query_iter(other).contains(led),
      LedQ::Take(other, _) => self.query_iter(other).contains(led),
    }
  }

  pub fn query_iter<'a>(&'a self, query: &'a LedQ) -> Box<dyn Iterator<Item = &'a LED> + 'a> {
    match query {
      LedQ::Name(name) => Box::new(self.by_name.get(name).into_iter()),
      LedQ::Names(names) => Box::new(names.iter().filter_map(|n| self.by_name.get(n))),
      // An LED several of them reach comes once, at the first of them
      LedQ::Or(queries) => Box::new(
        queries
          .iter()
          .flat_map(|q| self.query_iter(q))
          .unique_by(|led| led.name.as_str()),
      ),
      LedQ::Skip(other, n) => Box::new(self.query_iter(other).skip(*n)),
      LedQ::Take(other, n) => Box::new(self.query_iter(other).take(*n)),
      LedQ::Reverse(other) => {
        let mut items: Vec<_> = self.query_iter(other).collect();
        items.reverse();
        Box::new(items.into_iter())
      }
      _ => {
        let mut matches: Vec<&LED> = self
          .by_name
          .values()
          .filter_map(|led| {
            if self.matches(query, led) {
              Some(led)
            } else {
              None
            }
          })
          .collect();

        // For other non-ordered results (e.g. by take) sort by hardware address to maintain consistent order
        matches.sort_by_key(|led| {
          (
            led.address.board(),
            led.address.breakout(),
            led.address.index,
          )
        });
        Box::new(matches.into_iter())
      }
    }
  }

  pub fn config(&self, name: &'static str) -> Option<&LedConfiguration> {
    self
      .by_name
      .get(name)
      .and_then(|led| self.configs.get(&led.name))
  }

  /// The channel order of the LED at `index` on a board's breakout, or the default for an LED that isn't defined
  pub(crate) fn channels_at(&self, board: u8, breakout: Option<u8>, index: u8) -> LedChannels {
    self
      .channels_by_index
      .get(&(board, breakout, index))
      .copied()
      .unwrap_or_default()
  }

  /// The index and channel order of every LED defined on a board's breakout
  pub(crate) fn channels_on(
    &self,
    board: u8,
    breakout: Option<u8>,
  ) -> impl Iterator<Item = (u8, LedChannels)> + '_ {
    self
      .channels_by_index
      .iter()
      .filter(move |((b, bo, _), _)| *b == board && *bo == breakout)
      .map(|((_, _, index), channels)| (*index, *channels))
  }
}

impl Deref for LedLookup {
  type Target = HashMap<String, LED>;

  fn deref(&self) -> &Self::Target {
    &self.by_name
  }
}

impl DerefMut for LedLookup {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.by_name
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// LEDs named by their (board, breakout, index)
  fn lookup(addresses: &[(u8, Option<u8>, u8)]) -> LedLookup {
    let mut leds = LedLookup::default();
    for &(board, breakout, index) in addresses {
      let name = format!("{board}.{}.{index}", breakout.unwrap_or(0));
      let address = LedAddress::new(ExpAddress::new(board, breakout, 0), index);
      leds.insert(name.clone(), LED { name, address, ..Default::default() });
    }
    leds
  }

  fn names<'a>(leds: &'a LedLookup, query: &'a LedQ) -> Vec<&'a str> {
    leds.query_iter(query).map(|led| led.name.as_str()).collect()
  }

  #[test]
  fn leds_come_in_hardware_order() {
    let leds = lookup(&[(2, Some(0), 0), (1, Some(1), 1), (1, Some(1), 0), (1, Some(0), 5)]);
    assert_eq!(names(&leds, &LedQ::every()), ["1.0.5", "1.1.0", "1.1.1", "2.0.0"]);
  }

  #[test]
  fn hardware_order_holds_on_board_zero() {
    let leds = lookup(&[(0, None, 2), (0, None, 0), (0, None, 1)]);
    assert_eq!(names(&leds, &LedQ::every()), ["0.0.0", "0.0.1", "0.0.2"]);
  }

  #[test]
  fn a_tag_named_comes_in_hardware_order() {
    let mut leds = lookup(&[(2, Some(0), 0), (1, Some(1), 0), (1, Some(0), 5)]);
    for name in ["2.0.0", "1.0.5"] {
      leds.get_mut(name).unwrap().tags.push(Box::new(crate::tags::Insert));
    }
    leds.get_mut("1.1.0").unwrap().tags.push(Box::new(crate::tags::Lane));
    assert_eq!(names(&leds, &leds.tag_named("Insert")), ["1.0.5", "2.0.0"]);
  }

  #[test]
  fn or_runs_a_query_at_a_time_and_reaches_an_led_once() {
    let mut leds = lookup(&[(2, Some(0), 0), (1, Some(1), 0), (1, Some(0), 5)]);
    for name in ["2.0.0", "1.1.0"] {
      leds.get_mut(name).unwrap().tags.push(Box::new(crate::tags::Lane));
    }
    for name in ["1.0.5", "1.1.0"] {
      leds.get_mut(name).unwrap().tags.push(Box::new(crate::tags::Insert));
    }
    let lane_first = LedQ::Or(vec![leds.tag_named("Lane"), leds.tag_named("Insert")]);
    assert_eq!(names(&leds, &lane_first), ["1.1.0", "2.0.0", "1.0.5"]);
  }

  #[test]
  fn names_come_in_the_order_given() {
    let leds = lookup(&[(1, Some(0), 0), (1, Some(0), 1), (1, Some(0), 2)]);
    let query = LedQ::names(["1.0.2", "1.0.0", "1.0.1"]);
    assert_eq!(names(&leds, &query), ["1.0.2", "1.0.0", "1.0.1"]);
  }

  #[test]
  fn a_tag_no_led_has_reaches_none() {
    let leds = lookup(&[(1, Some(0), 0)]);
    assert!(names(&leds, &leds.tag_named("Insert")).is_empty());
  }

  #[test]
  fn reverse_is_hardware_order_backwards() {
    let leds = lookup(&[(1, Some(0), 5), (1, Some(1), 0), (2, Some(0), 0)]);
    assert_eq!(names(&leds, &LedQ::every().reverse()), ["2.0.0", "1.1.0", "1.0.5"]);
  }
}
