use std::collections::HashMap;

use crate::ResolvedLedPort;
use crate::hardware::exp::LedPort;

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpansionBoard {
  pub address: u8,
  pub breakout: Option<u8>,
  pub hardware_led_port_count: Option<u8>,
  pub led_ports: HashMap<u8, LedPort>,
}

#[derive(Debug, Clone)]
pub struct ResolvedExpansionBoard {
  pub address: u8,
  pub breakout: Option<u8>,
  pub led_ports: Vec<ResolvedLedPort>,
}

impl ExpansionBoard {
  pub fn new(address: &'static str, led_port_count: Option<u8>, breakout: Option<u8>) -> Self {
    Self {
      address: u8::from_str_radix(address, 16).unwrap(),
      breakout,
      hardware_led_port_count: led_port_count,
      led_ports: HashMap::new(),
    }
  }

  pub fn neutron() -> Self {
    Self::new("48", Some(4), None)
  }

  // TODO: fp_exp0051
  /// 2 DC motors, 127 LEDs

  /// 2 stepper, 128 LEDs
  pub fn fp_exp0061(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "90",
      (JumperState::Closed, JumperState::Open) => "91",
      (JumperState::Open, JumperState::Closed) => "92",
      (JumperState::Closed, JumperState::Closed) => "93",
    };

    Self::new(address, Some(4), None)
  }

  /// 4 servos, 128 LEDs
  pub fn fp_exp0071(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "B4",
      (JumperState::Closed, JumperState::Open) => "B5",
      (JumperState::Open, JumperState::Closed) => "B6",
      (JumperState::Closed, JumperState::Closed) => "B7",
    };

    Self::new(address, Some(4), None)
  }

  /// 256 LEDs
  pub fn fp_exp0081(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "84",
      (JumperState::Closed, JumperState::Open) => "85",
      (JumperState::Open, JumperState::Closed) => "86",
      (JumperState::Closed, JumperState::Closed) => "87",
    };

    Self::new(address, Some(8), None)
  }

  pub fn fp_exp0091(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "88",
      (JumperState::Closed, JumperState::Open) => "89",
      (JumperState::Open, JumperState::Closed) => "8A",
      (JumperState::Closed, JumperState::Closed) => "8B",
    };

    Self::new(address, Some(4), None)
  }

  /// shaker motor
  pub fn fp_exp1313(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "30",
      (JumperState::Closed, JumperState::Open) => "31",
      (JumperState::Open, JumperState::Closed) => "32",
      (JumperState::Closed, JumperState::Closed) => "33",
    };

    Self::new(address, None, None)
  }

  pub fn port(mut self, index: u8, port: LedPort) -> Self {
    // Verify this port is valid
    if self.hardware_led_port_count.is_none() {
      panic!(
        "Cannot add LED port to board {:X} because it does not support LED ports",
        self.address
      );
    } else if index > self.hardware_led_port_count.unwrap() {
      panic!(
        "LED port index {} exceeds hardware limit of {} for board {:X}",
        index,
        self.hardware_led_port_count.unwrap(),
        self.address
      );
    } else if self.led_ports.contains_key(&index) {
      panic!(
        "LED port index {} is already occupied on board {:X}",
        index, self.address
      );
    }

    self.led_ports.insert(index, port);
    self
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JumperState {
  Open,
  Closed,
}
