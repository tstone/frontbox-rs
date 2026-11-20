use std::borrow::Cow;

use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct DriverDefinition {
  pub name: &'static str,
  pub tags: Vec<Box<dyn Tag>>,
  pub location: Option<Vec3>,
  pub mode: Option<DriverMode>,
  /// For a driver powering a bulb ([`DriverDefinition::light`]), the bulb's color. A driver with one is also a light,
  /// named the same, that light management turns on and off.
  pub bulb: Option<Rgb>,
}

impl DriverDefinition {
  pub fn new(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name)
  }

  /// A driver powering a bulb, such as a lamp on a low-voltage driver. It's also a light, of the same name, which
  /// light management turns on and off over the driver's virtual switch.
  pub fn light(name: &'static str) -> DriverLightBuilder {
    DriverLightBuilder::new(name)
  }

  pub fn configs(&self) -> Vec<&ConfigDefinition> {
    match &self.mode {
      Some(mode) => mode.configs(),
      None => Vec::new(),
    }
  }

  pub fn q(&self) -> DriverQ {
    DriverQ::name(self.name)
  }

  /// Counts the driver's fires, keyed `driver_fires.<name>` for taps from software and `driver_fires_switch.<name>`
  /// for fires by the hardware from its trigger switch. Kept apart so a chattering switch shows. Leaks the keys, so
  /// build them once per driver at boot.
  pub(crate) fn fire_counters(&self) -> DriverFireCounters {
    let tapped = Box::leak(format!("driver_fires.{}", self.name).into_boxed_str());
    let switch_label = Box::leak(format!("{} (switch)", self.name).into_boxed_str());
    let switched = Box::leak(format!("driver_fires_switch.{}", self.name).into_boxed_str());
    DriverFireCounters {
      tapped: Counter::new(self.name, "Times software fired the driver")
        .key(tapped)
        .category("Drivers"),
      switched: Counter::new(
        switch_label,
        "Times the hardware fired the driver from its switch",
      )
      .key(switched)
      .category("Drivers"),
    }
  }
}

/// A driver's fire counters. See [`DriverDefinition::fire_counters`].
#[derive(Clone)]
pub(crate) struct DriverFireCounters {
  pub tapped: Counter,
  pub switched: Counter,
}

impl HardwareDefinition for DriverDefinition {
  fn name(&self) -> Cow<'static, str> {
    Cow::Borrowed(self.name)
  }

  fn tags(&self) -> Vec<Box<dyn Tag>> {
    self.tags.clone()
  }

  fn location(&self) -> Option<Vec3> {
    self.location
  }
}

impl Into<&'static str> for DriverDefinition {
  fn into(self) -> &'static str {
    self.name
  }
}

impl From<&DriverDefinition> for &'static str {
  fn from(value: &DriverDefinition) -> Self {
    value.name
  }
}

pub struct DriverDefinitionBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  location: Option<Vec3>,
  mode: Option<DriverMode>,
}

impl DriverDefinitionBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      location: None,
      mode: None,
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

  pub fn location(mut self, location: Vec3) -> Self {
    self.location = Some(location);
    self
  }

  pub fn mode(mut self, mode: DriverMode) -> Self {
    self.mode = Some(mode);
    self
  }

  pub fn build(self) -> DriverDefinition {
    DriverDefinition {
      name: self.name,
      tags: self.tags,
      location: self.location,
      mode: self.mode,
      bulb: None,
    }
  }
}

/// Builds a driver light. See [`DriverDefinition::light`].
pub struct DriverLightBuilder {
  name: &'static str,
  tags: Vec<Box<dyn Tag>>,
  location: Option<Vec3>,
  color: Rgb,
}

impl DriverLightBuilder {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      tags: Vec::new(),
      location: None,
      color: Rgb::new(255, 255, 255),
    }
  }

  /// The bulb's color, white by default. It only says what the light looks like, such as in the console's previews.
  pub fn color(mut self, color: Rgb) -> Self {
    self.color = color;
    self
  }

  pub fn tag(mut self, tag: impl Tag + 'static) -> Self {
    self.tags.push(Box::new(tag));
    self
  }

  pub fn tags(mut self, tags: impl IntoIterator<Item = Box<dyn Tag>>) -> Self {
    self.tags.extend(tags);
    self
  }

  pub fn location(mut self, location: Vec3) -> Self {
    self.location = Some(location);
    self
  }

  /// Held on at full power while its virtual switch is on, and off while it's off
  pub fn build(self) -> DriverDefinition {
    DriverDefinition {
      name: self.name,
      tags: self.tags,
      location: self.location,
      mode: Some(
        DriverMode::pulse_hold()
          .trigger_mode(DriverTriggerMode::VirtualSwitchTrue)
          .initial_pwm_power(Power::ZERO)
          .secondary_pwm_power(Power::FULL)
          .build(),
      ),
      bulb: Some(self.color),
    }
  }
}
