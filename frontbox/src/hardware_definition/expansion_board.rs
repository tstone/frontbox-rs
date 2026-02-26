use fast_protocol::*;

/// https://fastpinball.com/programming/exp/#expansion-board-addresses
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ExpansionBoardSpec {
  pub(crate) address: u8,
  pub(crate) breakout: Option<u8>,
  pub(crate) led_ports: Vec<LedPortSpec>,
}

impl ExpansionBoardSpec {
  pub fn custom(address: &'static str, breakout: Option<u8>) -> Self {
    Self {
      address: u8::from_str_radix(address, 16).unwrap(),
      breakout,
      led_ports: Vec::new(),
    }
  }

  pub fn neutron() -> Self {
    Self::custom("48", None)
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

    Self::custom(address, None)
  }

  /// 4 servos, 128 LEDs
  pub fn fp_exp0071(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "B4",
      (JumperState::Closed, JumperState::Open) => "B5",
      (JumperState::Open, JumperState::Closed) => "B6",
      (JumperState::Closed, JumperState::Closed) => "B7",
    };

    Self::custom(address, None)
  }

  /// 256 LEDs
  pub fn fp_exp0081(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "84",
      (JumperState::Closed, JumperState::Open) => "85",
      (JumperState::Open, JumperState::Closed) => "86",
      (JumperState::Closed, JumperState::Closed) => "87",
    };

    Self::custom(address, None)
  }

  pub fn fp_exp0091(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "88",
      (JumperState::Closed, JumperState::Open) => "89",
      (JumperState::Open, JumperState::Closed) => "8A",
      (JumperState::Closed, JumperState::Closed) => "8B",
    };

    Self::custom(address, None)
  }

  /// shaker motor
  pub fn fp_exp1313(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "30",
      (JumperState::Closed, JumperState::Open) => "31",
      (JumperState::Open, JumperState::Closed) => "32",
      (JumperState::Closed, JumperState::Closed) => "33",
    };

    Self::custom(address, None)
  }

  pub fn with_led_port(mut self, port: LedPortSpec) -> Self {
    self.led_ports.push(port);
    self
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JumperState {
  Open,
  Closed,
}

#[derive(Debug, Clone)]
pub struct LedPortSpec {
  pub port: u8,
  pub start: u8,
  pub leds: Vec<&'static str>,
  pub led_type: LedType,
}

impl Default for LedPortSpec {
  fn default() -> Self {
    Self {
      port: 0,
      start: 0,
      leds: Vec::new(),
      led_type: LedType::WS2812,
    }
  }
}
