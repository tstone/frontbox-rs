use std::time::Duration;

use fast_protocol::net::prelude::Power;

pub trait ConfigDisplay {
  fn config_display(&self) -> String;
}

impl ConfigDisplay for Duration {
  fn config_display(&self) -> String {
    humantime::format_duration(*self).to_string()
  }
}

impl ConfigDisplay for Power {
  fn config_display(&self) -> String {
    format!("{}%", ((self.power as f32 / 255.0) * 100.0) as u8)
  }
}

impl ConfigDisplay for u8 {
  fn config_display(&self) -> String {
    format!("{}", self)
  }
}

impl ConfigDisplay for i8 {
  fn config_display(&self) -> String {
    format!("{}", self)
  }
}

impl ConfigDisplay for u16 {
  fn config_display(&self) -> String {
    format!("{}", self)
  }
}

impl ConfigDisplay for i16 {
  fn config_display(&self) -> String {
    format!("{}", self)
  }
}
