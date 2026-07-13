use crate::prelude::*;

pub struct LedStripBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  count: u16,
  plane: Option<&'static ReferencePlane>,
  locations: Vec<Vec2>, // local offsets within `plane`
  config: Option<LedConfiguration>,
}

impl LedStripBuilder {
  /// Strip origin given directly in absolute space, unrotated -- a
  /// convenience for the common case where you don't need a named plane.
  pub fn new(name: &'static str, count: u16) -> Self {
    Self {
      name,
      count,
      tags: Vec::new(),
      plane: None,
      locations: Vec::new(),
      config: None,
    }
  }

  pub fn tag(mut self, tag: impl Tag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn tags(mut self, tags: impl IntoIterator<Item = Box<dyn Tag>>) -> Self {
    self.tags.extend(tags);
    self
  }

  /// Starting at the reference plane's origin and extending in even increments until end coordinate
  pub fn locations(mut self, plane: &'static ReferencePlane, direction: LedStripDirection) -> Self {
    self.plane = Some(plane);
    let start = Vec2::new(plane.origin.x, plane.origin.y);
    let end = plane.extent;

    match self.count {
      0 => {}
      1 => self.locations.push(start),
      _ => {
        let step = (end - start) / (self.count as f32 - 1.0);
        self
          .locations
          .extend((0..self.count).map(|i| match direction {
            LedStripDirection::Forwards => start + step * i as f32,
            LedStripDirection::Backwards => end.x - (start + step * i as f32),
          }));
      }
    }
    self
  }

  pub fn config(mut self, config: LedConfiguration) -> Self {
    self.config = Some(config);
    self
  }

  pub fn channels(mut self, channels: LedChannels) -> Self {
    self.config_mut().channels = channels;
    self
  }

  fn config_mut(&mut self) -> &mut LedConfiguration {
    self.config.get_or_insert_with(LedConfiguration::default)
  }

  pub fn build(self) -> MultiLedDefinition {
    let plane = if self.locations.len() > 0 {
      self.plane.unwrap()
    } else {
      &ReferencePlane::default()
    };

    let locations = self
      .locations
      .into_iter()
      .map(|p| p.relative_to(plane))
      .collect();

    MultiLedDefinition::new(self.name, self.tags, self.count, locations, self.config)
  }
}

pub enum LedStripDirection {
  Forwards,
  Backwards,
}
