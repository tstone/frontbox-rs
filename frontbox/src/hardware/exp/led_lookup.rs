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

  fn matches(&self, query: &LedQuery, led: &LED) -> bool {
    match query {
      LedQuery::Every => true,
      LedQuery::Name(name) => led.name == *name,
      LedQuery::Names(names) => names.contains(&led.name),
      LedQuery::Tag(tag) => led.has_typed_tag(*tag),
      LedQuery::And(qs) => qs.iter().all(|q| self.matches(q, led)),
      LedQuery::Or(qs) => qs.iter().any(|q| self.matches(q, led)),
      LedQuery::Location(plane, region) => led
        .location
        .map(|location| region.within(plane.to_relative(location)))
        .unwrap_or(false),
      LedQuery::Reverse(other) => self.matches(other, led),
      LedQuery::Skip(other, _) => self.query_iter(other).contains(led),
      LedQuery::Take(other, _) => self.query_iter(other).contains(led),
    }
  }

  pub fn query_iter<'a>(&'a self, query: &'a LedQuery) -> Box<dyn Iterator<Item = &'a LED> + 'a> {
    match query {
      LedQuery::Name(name) => Box::new(self.by_name.get(name).into_iter()),
      LedQuery::Names(names) => Box::new(names.iter().filter_map(|n| self.by_name.get(n))),
      LedQuery::Or(queries) => Box::new(queries.iter().flat_map(|q| self.query_iter(q))),
      LedQuery::Skip(other, n) => Box::new(self.query_iter(other).skip(*n)),
      LedQuery::Take(other, n) => Box::new(self.query_iter(other).take(*n)),
      LedQuery::Reverse(other) => {
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
