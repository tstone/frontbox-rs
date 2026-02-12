use bitflags::bitflags;

pub struct DriverTriggerBuilder {
  flags: DriverTrigger,
}

impl DriverTriggerBuilder {
  pub fn new() -> Self {
    Self {
      flags: DriverTrigger::empty(),
    }
  }

  pub fn enabled(mut self, enabled: bool) -> Self {
    if enabled {
      self.flags.insert(DriverTrigger::ENABLED);
    } else {
      self.flags.remove(DriverTrigger::ENABLED);
    }
    self
  }

  pub fn one_shot(mut self, one_shot: bool) -> Self {
    if one_shot {
      self.flags.insert(DriverTrigger::ONE_SHOT);
    } else {
      self.flags.remove(DriverTrigger::ONE_SHOT);
    }
    self
  }

  pub fn invert_switch1(mut self, invert: &Option<bool>) -> Self {
    if let Some(true) = invert {
      self.flags.insert(DriverTrigger::INVERT_SWITCH1);
    } else {
      self.flags.remove(DriverTrigger::INVERT_SWITCH1);
    }
    self
  }

  pub fn disable_switch(mut self, disable: bool) -> Self {
    if disable {
      self.flags.insert(DriverTrigger::DISABLE_SWITCH);
    } else {
      self.flags.remove(DriverTrigger::DISABLE_SWITCH);
    }
    self
  }

  pub fn bits(self) -> u8 {
    self.flags.bits()
  }
}

bitflags! {
  /// https://fastpinball.com/fast-serial-protocol/net/drivers/#controlling-how-drivers-fire-with-trigger-flags
  pub struct DriverTrigger: u8 {
    const ENABLED = 0b00000001;
    const UNUSED1 = 0b00000010;
    const UNUSED2 = 0b00000100;
    const ONE_SHOT = 0b00001000;
    const INVERT_SWITCH1 = 0b00010000;
    const INVERT_SWITCH2 = 0b00100000;
    const MANUAL = 0b01000000;
    const DISABLE_SWITCH = 0b10000000;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_driver_trigger_bitflags() {
    let trigger = DriverTrigger::ENABLED | DriverTrigger::DISABLE_SWITCH;
    assert_eq!(trigger.bits(), 0b10000001);
    assert_eq!(format!("{:X}", trigger.bits()), "81");

    let trigger =
      DriverTrigger::ENABLED | DriverTrigger::INVERT_SWITCH1 | DriverTrigger::DISABLE_SWITCH;
    assert_eq!(trigger.bits(), 0b10010001);
    assert_eq!(format!("{:X}", trigger.bits()), "91");
  }
}
