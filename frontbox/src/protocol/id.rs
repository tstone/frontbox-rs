use crate::protocol::common::expansion_addr;
use crate::protocol::prelude::*;

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

impl FastCommand for IdCommand {
  type Response = IdResponse;

  fn prefix() -> &'static str {
    "id"
  }

  fn to_string(&self) -> String {
    match self.address {
      Some(FastAddress::Io(id)) => format!("ID@{}:\r", id),
      Some(FastAddress::Exp(board, breakout)) => {
        format!("ID@{}:\r", expansion_addr(board, breakout))
      }
      None => "ID:\r".to_string(),
    }
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
    let product_number = parts[1].trim().to_string();
    let firmware_version = parts[2].trim().to_string();
    Ok(IdResponse::Report {
      processor,
      product_number,
      firmware_version,
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdResponse {
  Report {
    processor: String,
    product_number: String,
    firmware_version: String,
  },
  Failed,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_success() {
    let data = "FP-CPU-002  3208 2.00";
    let result = IdCommand::new().parse(RawResponse {
      prefix: "ID:".to_string(),
      payload: data.to_string(),
      ..Default::default()
    });

    assert!(result.is_ok());
    match result.unwrap() {
      IdResponse::Report {
        processor,
        product_number,
        firmware_version,
      } => {
        assert_eq!(processor, "FP-CPU-002");
        assert_eq!(product_number, "3208");
        assert_eq!(firmware_version, "2.00");
      }
      _ => panic!("Expected IdResponse"),
    }
  }
}
