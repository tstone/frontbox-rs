use crate::prelude::*;

/// Where a light is wired, which is also what names it: no two lights share an address.
///
/// Light management and its callers name a light by its address. Within a run a light's address never changes, but
/// anything saved, such as a scene file, names lights by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "kind")]
pub enum LightAddress {
  /// Written over the EXP bus as part of an LED address's RGB
  Exp {
    led: LedAddress,
    channels: LedChannels,
  },
  /// A driver powering a bulb, turned on and off over its virtual switch
  Driver { driver: &'static str },
}

impl LightAddress {
  /// An RGB LED, using all three channels of its LED address
  pub fn rgb(led: LedAddress) -> Self {
    Self::Exp {
      led,
      channels: LedChannels::Rgb,
    }
  }

  /// The LED address it's written to, if it's written over the EXP bus
  pub fn led(&self) -> Option<&LedAddress> {
    match self {
      Self::Exp { led, .. } => Some(led),
      Self::Driver { .. } => None,
    }
  }

  /// The driver it's written to, if it's a driver light
  pub fn driver(&self) -> Option<&'static str> {
    match self {
      Self::Driver { driver } => Some(driver),
      Self::Exp { .. } => None,
    }
  }

  /// The order lights come in when nothing else orders them: LEDs by board, breakout, then index, and then driver
  /// lights by driver name
  pub(crate) fn hardware_order(&self) -> (bool, (u8, Option<u8>, u8), &'static str) {
    match self {
      Self::Exp { led, .. } => (false, (led.board(), led.breakout(), led.index), ""),
      Self::Driver { driver } => (true, (0, None, 0), driver),
    }
  }
}

impl Default for LightAddress {
  fn default() -> Self {
    Self::rgb(LedAddress::default())
  }
}

/// Which channels of an LED address a light uses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum LedChannels {
  /// All three, as one color (an RGB LED)
  Rgb,
  /// One of them, as one brightness
  One(LedChannel),
}

/// One channel of an LED address, by its position on the wire
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum LedChannel {
  R,
  G,
  B,
}

impl LedChannel {
  /// In order on the wire
  pub const ALL: [LedChannel; 3] = [LedChannel::R, LedChannel::G, LedChannel::B];
}

impl Contextual<Vec<LightAddress>> for LightQ {
  fn resolve(&self, ctx: &SystemContext) -> Vec<LightAddress> {
    self.query_addresses(ctx.into())
  }
}

impl Contextual<Vec<LightAddress>> for Vec<LightQ> {
  fn resolve(&self, ctx: &SystemContext) -> Vec<LightAddress> {
    self
      .iter()
      .flat_map(|q| q.query_addresses(ctx.into()))
      .collect()
  }
}

impl Contextual<Vec<LightAddress>> for Vec<&LightQ> {
  fn resolve(&self, ctx: &SystemContext) -> Vec<LightAddress> {
    self
      .iter()
      .flat_map(|q| q.query_addresses(ctx.into()))
      .collect()
  }
}

impl Contextual<Vec<LightAddress>> for Vec<LightAddress> {
  fn resolve(&self, _ctx: &SystemContext) -> Vec<LightAddress> {
    self.clone()
  }
}

impl Contextual<Vec<LightAddress>> for Box<dyn Contextual<Vec<LightAddress>> + Send + Sync> {
  fn resolve(&self, ctx: &SystemContext) -> Vec<LightAddress> {
    (**self).resolve(ctx)
  }
}
