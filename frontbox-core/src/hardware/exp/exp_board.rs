use std::collections::HashMap;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum FastExpansionBoardModels {
  Neuron,
  FpExp0051,
  FpExp0061,
  FpExp0071,
  FpExp0081,
  FpExp0091,
  FpExp1313,
  Custom,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct ExpBoard {
  pub address: u8,
  pub breakout: Option<u8>,
  pub hardware_led_port_count: Option<u8>,
  pub led_ports: HashMap<u8, LedPort>,
  pub model: FastExpansionBoardModels,
}

#[derive(Debug, Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ResolvedExpansionBoard {
  pub address: u8,
  pub breakout: Option<u8>,
  pub led_ports: Vec<ResolvedLedPort>,
  pub model: FastExpansionBoardModels,
}

impl ExpBoard {
  pub fn new(
    address: &'static str,
    led_port_count: Option<u8>,
    breakout: Option<u8>,
    model: FastExpansionBoardModels,
  ) -> Self {
    Self {
      address: u8::from_str_radix(address, 16).unwrap(),
      breakout,
      hardware_led_port_count: led_port_count,
      led_ports: HashMap::new(),
      model,
    }
  }

  pub fn neuron() -> Self {
    Self::new("48", Some(4), None, FastExpansionBoardModels::Neuron)
  }

  /// 2 DC motors, 128 LEDs
  pub fn fp_exp0051(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      // TODO: confirm
      (JumperState::Open, JumperState::Open) => "D0",
      (JumperState::Closed, JumperState::Open) => "D1",
      (JumperState::Open, JumperState::Closed) => "D2",
      (JumperState::Closed, JumperState::Closed) => "D3",
    };

    Self::new(address, Some(4), None, FastExpansionBoardModels::FpExp0061)
  }

  /// 2 stepper, 128 LEDs
  pub fn fp_exp0061(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "90",
      (JumperState::Closed, JumperState::Open) => "91",
      (JumperState::Open, JumperState::Closed) => "92",
      (JumperState::Closed, JumperState::Closed) => "93",
    };

    Self::new(address, Some(4), None, FastExpansionBoardModels::FpExp0061)
  }

  /// 4 servos, 128 LEDs
  pub fn fp_exp0071(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "B4",
      (JumperState::Closed, JumperState::Open) => "B5",
      (JumperState::Open, JumperState::Closed) => "B6",
      (JumperState::Closed, JumperState::Closed) => "B7",
    };

    Self::new(address, Some(4), None, FastExpansionBoardModels::FpExp0071)
  }

  /// 256 LEDs
  pub fn fp_exp0081(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "84",
      (JumperState::Closed, JumperState::Open) => "85",
      (JumperState::Open, JumperState::Closed) => "86",
      (JumperState::Closed, JumperState::Closed) => "87",
    };

    Self::new(address, Some(8), None, FastExpansionBoardModels::FpExp0081)
  }

  pub fn fp_exp0091(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "88",
      (JumperState::Closed, JumperState::Open) => "89",
      (JumperState::Open, JumperState::Closed) => "8A",
      (JumperState::Closed, JumperState::Closed) => "8B",
    };

    Self::new(address, Some(4), None, FastExpansionBoardModels::FpExp0091)
  }

  /// shaker motor
  pub fn fp_exp1313(jumper_0: JumperState, jumper_1: JumperState) -> Self {
    let address = match (jumper_0, jumper_1) {
      (JumperState::Open, JumperState::Open) => "30",
      (JumperState::Closed, JumperState::Open) => "31",
      (JumperState::Open, JumperState::Closed) => "32",
      (JumperState::Closed, JumperState::Closed) => "33",
    };

    Self::new(address, None, None, FastExpansionBoardModels::FpExp1313)
  }

  /// * `port_num` reflects the port printed on the PCB. e.g. "PORT 1" on the PCB is specified as `1` here
  pub fn wire_led_port(mut self, port_num: u8, port: LedPort) -> Self {
    // Verify this port is valid
    let Some(port_count) = self.hardware_led_port_count else {
      panic!(
        "Cannot add LED port to board {:X} because it does not support LED ports",
        self.address
      );
    };

    if port_num == 0 || port_num > port_count {
      panic!(
        "LED port {} is not on board {:X}, which has ports 1-{} (port numbers match the PCB, starting at 1)",
        port_num, self.address, port_count
      );
    }

    // ports are stored 0-based
    let port_idx = port_num - 1;
    if self.led_ports.contains_key(&port_idx) {
      panic!(
        "LED port {} is already occupied on board {:X}",
        port_num, self.address
      );
    }

    self.led_ports.insert(port_idx, port);
    self
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JumperState {
  Open,
  Closed,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ports_are_stored_zero_based() {
    let board = ExpBoard::neuron()
      .wire_led_port(2, LedPort::ws2812())
      .wire_led_port(1, LedPort::ws2812());
    assert!(board.led_ports.contains_key(&0));
    assert!(board.led_ports.contains_key(&1));
  }

  #[test]
  #[should_panic(expected = "already occupied")]
  fn panics_on_duplicate_port() {
    ExpBoard::neuron()
      .wire_led_port(1, LedPort::ws2812())
      .wire_led_port(1, LedPort::ws2812());
  }

  #[test]
  #[should_panic(expected = "starting at 1")]
  fn panics_on_port_zero() {
    ExpBoard::neuron().wire_led_port(0, LedPort::ws2812());
  }

  #[test]
  #[should_panic(expected = "has ports 1-4")]
  fn panics_on_port_past_hardware_count() {
    ExpBoard::neuron().wire_led_port(5, LedPort::ws2812());
  }
}
