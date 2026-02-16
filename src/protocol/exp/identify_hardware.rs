use crate::protocol::common::expansion_addr;
use crate::protocol::prelude::*;

pub struct IdentifyHardwareCommand {
  expansion_board: u8,
  breakout: u8,
}

impl IdentifyHardwareCommand {
  pub fn new(expansion_board: u8, breakout: u8) -> Self {
    Self {
      expansion_board,
      breakout,
    }
  }
}

impl FastCommand for IdentifyHardwareCommand {
  type Response = ExpansionBreakoutInfo;

  fn prefix() -> &'static str {
    "ih"
  }

  fn to_string(&self) -> String {
    let address = expansion_addr(self.expansion_board, Some(self.breakout));
    format!("IH@{}:\r", address)
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    // Example: IH:480,AL,P04,R80
    let segments = raw.payload.split(',').collect::<Vec<&str>>();

    if segments[1] == "X" {
      return Ok(ExpansionBreakoutInfo::inactive(
        self.expansion_board,
        self.breakout,
      ));
    }

    let local = segments[1].contains("L");
    let mut led_ports: u8 = 0;
    let mut leds: u16 = 0;
    let mut devices: u8 = 0;

    for segment in &segments[2..] {
      match segment.chars().next().unwrap_or(' ') {
        'P' => {
          led_ports = u8::from_str_radix(&segment[1..], 16).unwrap_or(0);
        }
        'R' => {
          leds = u16::from_str_radix(&segment[1..], 16).unwrap_or(0);
        }
        'D' => {
          devices = u8::from_str_radix(&segment[1..], 16).unwrap_or(0);
        }
        _ => {
          log::warn!("Unknown segment in IdentifyHardware response: {}", segment);
        }
      }
    }

    Ok(ExpansionBreakoutInfo::active(
      self.expansion_board,
      self.breakout,
      local,
      led_ports,
      leds,
      devices,
    ))
  }
}

#[derive(Debug, Clone, Default)]
pub enum ExpansionBreakoutStatus {
  Active,
  #[default]
  Inactive,
}

#[derive(Debug, Clone)]
pub enum ExpansionBreakoutInfo {
  Inactive {
    expansion_board: u8,
    breakout: u8,
  },
  Active {
    expansion_board: u8,
    breakout: u8,
    local: bool,
    led_ports: u8,
    leds: u16,
    devices: u8,
  },
}

impl ExpansionBreakoutInfo {
  pub fn inactive(expansion_board: u8, breakout: u8) -> Self {
    Self::Inactive {
      expansion_board,
      breakout,
    }
  }

  pub fn active(
    expansion_board: u8,
    breakout: u8,
    local: bool,
    led_ports: u8,
    leds: u16,
    devices: u8,
  ) -> Self {
    Self::Active {
      expansion_board,
      breakout,
      local,
      led_ports,
      leds,
      devices,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_inactive() {
    let cmd = IdentifyHardwareCommand::new(72, 0);
    let raw_response = RawResponse {
      prefix: "IH".to_string(),
      payload: "480,X,R80".to_string(),
      ..Default::default()
    };
    let result = cmd.parse(raw_response).unwrap();
    match result {
      ExpansionBreakoutInfo::Inactive {
        expansion_board,
        breakout,
      } => {
        assert_eq!(expansion_board, 72);
        assert_eq!(breakout, 0);
      }
      _ => panic!("Expected Inactive variant"),
    }
  }

  #[test]
  fn test_active() {
    let cmd = IdentifyHardwareCommand::new(72, 0);
    let raw_response = RawResponse {
      prefix: "IH".to_string(),
      payload: "480,AL,P04,R80,D02".to_string(),
      ..Default::default()
    };
    let result = cmd.parse(raw_response).unwrap();
    match result {
      ExpansionBreakoutInfo::Active {
        expansion_board,
        breakout,
        local,
        led_ports,
        leds,
        devices,
      } => {
        assert_eq!(expansion_board, 72);
        assert_eq!(breakout, 0);
        assert!(local);
        assert_eq!(led_ports, 4);
        assert_eq!(leds, 128);
        assert_eq!(devices, 2);
      }
      _ => panic!("Expected Active variant"),
    }
  }
}
