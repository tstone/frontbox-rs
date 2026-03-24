use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum ConfigItem {
  String {
    current: String,
    default: String,
    options: Vec<String>,
    name: &'static str,
    description: &'static str,
  },
  Integer {
    value: i32,
    default: i32,
    min: Option<i32>,
    max: Option<i32>,
    name: &'static str,
    description: &'static str,
    units: Option<&'static str>,
  },
  Boolean {
    current: bool,
    default: bool,
    name: &'static str,
    description: &'static str,
  },
}

impl ConfigItem {
  pub fn name(&self) -> &'static str {
    match self {
      ConfigItem::String { name, .. } => name,
      ConfigItem::Integer { name, .. } => name,
      ConfigItem::Boolean { name, .. } => name,
    }
  }

  pub fn description(&self) -> &'static str {
    match self {
      ConfigItem::String { description, .. } => description,
      ConfigItem::Integer { description, .. } => description,
      ConfigItem::Boolean { description, .. } => description,
    }
  }
}
