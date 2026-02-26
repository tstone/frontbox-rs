#[derive(Debug, Clone)]
pub enum ConfigItem {
  String {
    current: String,
    default: String,
    options: Vec<String>,
    name: &'static str,
    description: &'static str,
  },
  Integer {
    current: i32,
    default: i32,
    min: i32,
    max: i32,
    name: &'static str,
    description: &'static str,
  },
  Boolean {
    current: bool,
    default: bool,
    name: &'static str,
    description: &'static str,
  },
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
  String(String),
  Integer(i32),
  Boolean(bool),
}

impl Into<ConfigValue> for String {
  fn into(self) -> ConfigValue {
    ConfigValue::String(self)
  }
}

impl Into<ConfigValue> for &str {
  fn into(self) -> ConfigValue {
    ConfigValue::String(self.to_string())
  }
}

impl Into<ConfigValue> for u8 {
  fn into(self) -> ConfigValue {
    ConfigValue::Integer(self as i32)
  }
}

impl Into<ConfigValue> for u16 {
  fn into(self) -> ConfigValue {
    ConfigValue::Integer(self as i32)
  }
}

impl Into<ConfigValue> for i32 {
  fn into(self) -> ConfigValue {
    ConfigValue::Integer(self)
  }
}

impl Into<ConfigValue> for bool {
  fn into(self) -> ConfigValue {
    ConfigValue::Boolean(self)
  }
}

impl ConfigValue {
  pub fn as_string(&self) -> Option<String> {
    if let ConfigValue::String(s) = self {
      Some(s.clone())
    } else {
      None
    }
  }

  pub fn as_str(&self) -> Option<&str> {
    if let ConfigValue::String(s) = self {
      Some(s)
    } else {
      None
    }
  }

  pub fn as_integer(&self) -> Option<i32> {
    if let ConfigValue::Integer(i) = self {
      Some(*i)
    } else {
      None
    }
  }

  pub fn as_usize(&self) -> Option<usize> {
    if let ConfigValue::Integer(i) = self {
      if *i >= 0 { Some(*i as usize) } else { None }
    } else {
      None
    }
  }

  pub fn as_u64(&self) -> Option<u64> {
    if let ConfigValue::Integer(i) = self {
      if *i >= 0 { Some(*i as u64) } else { None }
    } else {
      None
    }
  }

  pub fn as_u32(&self) -> Option<u32> {
    if let ConfigValue::Integer(i) = self {
      if *i >= 0 { Some(*i as u32) } else { None }
    } else {
      None
    }
  }

  pub fn as_u16(&self) -> Option<u16> {
    if let ConfigValue::Integer(i) = self {
      if *i >= 0 && *i <= u16::MAX as i32 {
        Some(*i as u16)
      } else {
        None
      }
    } else {
      None
    }
  }

  pub fn as_u8(&self) -> Option<u8> {
    if let ConfigValue::Integer(i) = self {
      if *i >= 0 && *i <= u8::MAX as i32 {
        Some(*i as u8)
      } else {
        None
      }
    } else {
      None
    }
  }

  pub fn as_boolean(&self) -> Option<bool> {
    if let ConfigValue::Boolean(b) = self {
      Some(*b)
    } else {
      None
    }
  }
}
