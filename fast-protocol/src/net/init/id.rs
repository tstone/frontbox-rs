use crate::common::expansion_addr;
use crate::*;

#[derive(Debug, Clone)]
pub struct IdCommand {
  address: Option<FastAddress>,
}

impl IdCommand {
  pub fn new() -> Self {
    IdCommand { address: None }
  }

  pub fn io(id: u8) -> Self {
    IdCommand {
      address: Some(FastAddress::Io(id)),
    }
  }

  pub fn exp(board: u8, breakout: Option<u8>) -> Self {
    IdCommand {
      address: Some(FastAddress::Exp(board, breakout)),
    }
  }
}

impl FastStringCommand for IdCommand {
  fn to_string(&self) -> String {
    match self.address {
      Some(FastAddress::Io(id)) => format!("ID@{}:\r", id),
      Some(FastAddress::Exp(board, breakout)) => {
        format!("ID@{}:\r", expansion_addr(board, breakout))
      }
      None => "ID:\r".to_string(),
    }
  }
}

impl FastRequestCommand for IdCommand {
  type Response = IdResponse;

  fn prefix() -> &'static str {
    "id"
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    let parts: Vec<&str> = raw
      .payload
      .split(' ')
      .filter(|part| !part.is_empty())
      .collect();
    if parts.len() != 3 {
      return Err(FastResponseError::InvalidFormat);
    }

    let processor = parts[0].trim().to_string();
    let mainboard_name = parts[1].trim().to_string();
    let firmware_version = parts[2].trim().to_string();
    Ok(IdResponse::Report {
      processor,
      mainboard_name,
      firmware_version,
    })
  }
}

impl Default for IdCommand {
  fn default() -> Self {
    Self::new()
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdResponse {
  Report {
    processor: String,
    mainboard_name: String,
    firmware_version: String,
  },
  Failed,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_success() {
    let data = "NET FP-SBI-0095  01.99";
    let result = IdCommand::new().parse(RawResponse {
      prefix: "ID:".to_string(),
      payload: data.to_string(),
      ..Default::default()
    });

    assert!(result.is_ok());
    match result.unwrap() {
      IdResponse::Report {
        processor,
        mainboard_name,
        firmware_version,
      } => {
        assert_eq!(processor, "NET");
        assert_eq!(mainboard_name, "FP-SBI-0095");
        assert_eq!(firmware_version, "01.99");
      }
      _ => panic!("Expected IdResponse"),
    }
  }
}
