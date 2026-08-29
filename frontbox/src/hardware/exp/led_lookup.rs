use itertools::Itertools;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::prelude::*;

#[derive(Debug, Clone, Default)]
pub struct LedLookup {
  by_name: HashMap<String, LED>,
  configs: HashMap<String, LedConfiguration>,
  channel_by_address: HashMap<LedAddress, LedChannels>,
}

impl LedLookup {
  pub fn new(boards: &Vec<ResolvedExpansionBoard>) -> Self {
    let mut by_name = HashMap::new();
    let mut configs = HashMap::new();
    let mut channel_by_address = HashMap::new();

    for board in boards {
      for port in &board.led_ports {
        for led in &port.leds {
          let address = LedAddress {
            exp: led.assignment.clone(),
            index: led.id as u16,
          };

          let channel = if let Some(config) = &led.definition.config {
            config.channels
          } else {
            LedChannels::default()
          };
          channel_by_address.insert(address.clone(), channel);

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
      channel_by_address,
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
      LedQ::Or(queries) => Box::new(queries.iter().flat_map(|q| self.query_iter(q))),
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
          ((led.address.board() as u32 * 10) + led.address.breakout().unwrap_or(0) as u32)
            * led.address.index as u32
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

  pub(crate) fn color_channels_by_id(&self, addr: &LedAddress) -> LedChannels {
    match self.channel_by_address.get(&addr) {
      Some(channels) => *channels,
      None => LedChannels::default(),
    }
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
