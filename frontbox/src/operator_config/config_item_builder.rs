use crate::operator_config::*;

pub trait OperatorConfigBuilder {
  fn build(self) -> (&'static str, ConfigItem);
}

pub struct OperatorConfigs;

impl OperatorConfigs {
  pub fn string(key: &'static str) -> StringConfigBuilder {
    StringConfigBuilder::new(key)
  }

  pub fn integer(key: &'static str) -> IntegerConfigBuilder {
    IntegerConfigBuilder::new(key)
  }

  pub fn boolean(key: &'static str) -> BooleanConfigBuilder {
    BooleanConfigBuilder::new(key)
  }
}

#[derive(Debug, Clone, Default)]
pub struct StringConfigBuilder {
  key: &'static str,
  current: String,
  default: String,
  options: Vec<String>,
  name: &'static str,
  description: &'static str,
}

impl StringConfigBuilder {
  pub fn new(key: &'static str) -> Self {
    Self {
      key,
      current: String::new(),
      default: String::new(),
      options: Vec::new(),
      name: "",
      description: "",
    }
  }

  pub fn default(mut self, default: String) -> Self {
    self.current = default.clone();
    self.default = default;
    self
  }

  pub fn options(mut self, options: Vec<String>) -> Self {
    self.options = options;
    self
  }

  pub fn name(mut self, name: &'static str) -> Self {
    self.name = name;
    self
  }

  pub fn description(mut self, description: &'static str) -> Self {
    self.description = description;
    self
  }
}

impl OperatorConfigBuilder for StringConfigBuilder {
  fn build(self) -> (&'static str, ConfigItem) {
    (
      self.key,
      ConfigItem::String {
        current: self.current,
        default: self.default,
        options: self.options,
        name: self.name,
        description: self.description,
      },
    )
  }
}

#[derive(Debug, Clone, Default)]
pub struct IntegerConfigBuilder {
  key: &'static str,
  value: i32,
  default: i32,
  min: Option<i32>,
  max: Option<i32>,
  name: &'static str,
  description: &'static str,
  units: Option<&'static str>,
}

impl IntegerConfigBuilder {
  pub fn new(key: &'static str) -> Self {
    Self {
      key,
      value: 0,
      default: 0,
      min: None,
      max: None,
      name: "",
      description: "",
      units: None,
    }
  }

  pub fn default(mut self, default: i32) -> Self {
    self.value = default;
    self.default = default;
    self
  }

  pub fn min(mut self, min: i32) -> Self {
    self.min = Some(min);
    self
  }

  pub fn max(mut self, max: i32) -> Self {
    self.max = Some(max);
    self
  }

  pub fn name(mut self, name: &'static str) -> Self {
    self.name = name;
    self
  }

  pub fn description(mut self, description: &'static str) -> Self {
    self.description = description;
    self
  }

  pub fn units(mut self, units: &'static str) -> Self {
    self.units = Some(units);
    self
  }
}

impl OperatorConfigBuilder for IntegerConfigBuilder {
  fn build(self) -> (&'static str, ConfigItem) {
    (
      self.key,
      ConfigItem::Integer {
        value: self.value,
        default: self.default,
        min: self.min,
        max: self.max,
        name: self.name,
        description: self.description,
        units: self.units,
      },
    )
  }
}

#[derive(Debug, Clone, Default)]
pub struct BooleanConfigBuilder {
  key: &'static str,
  current: bool,
  default: bool,
  name: &'static str,
  description: &'static str,
}

impl BooleanConfigBuilder {
  pub fn new(key: &'static str) -> Self {
    Self {
      key,
      current: false,
      default: false,
      name: "",
      description: "",
    }
  }

  pub fn default(mut self, default: bool) -> Self {
    self.current = default;
    self.default = default;
    self
  }

  pub fn name(mut self, name: &'static str) -> Self {
    self.name = name;
    self
  }

  pub fn description(mut self, description: &'static str) -> Self {
    self.description = description;
    self
  }
}

impl OperatorConfigBuilder for BooleanConfigBuilder {
  fn build(self) -> (&'static str, ConfigItem) {
    (
      self.key,
      ConfigItem::Boolean {
        current: self.current,
        default: self.default,
        name: self.name,
        description: self.description,
      },
    )
  }
}
