use crate::*;

#[derive(Debug, Clone)]
pub struct NodeNameCommand {
  id: u8,
}

impl NodeNameCommand {
  pub fn new(id: u8) -> Self {
    NodeNameCommand { id }
  }
}

impl FastStringCommand for NodeNameCommand {
  fn to_string(&self) -> String {
    format!("NN@{:X}:\r", self.id)
  }
}

impl FastRequestCommand for NodeNameCommand {
  type Response = NodeInfo;

  fn prefix() -> &'static str {
    "nn"
  }

  fn parse(&self, raw: RawResponse) -> Result<Self::Response, FastResponseError> {
    let parts: Vec<&str> = raw
      .payload
      .split(',')
      .filter(|part| !part.is_empty())
      .collect();
    if parts.len() != 11 {
      return Err(FastResponseError::InvalidFormat);
    }

    let node_id = parts[0]
      .trim()
      .parse::<u8>()
      .map_err(|_| FastResponseError::InvalidFormat)?;
    let name = parts[1].trim().to_string();
    let firmware_version = parts[2].trim().to_string();
    let driver_count = parts[3]
      .trim()
      .parse::<u16>()
      .map_err(|_| FastResponseError::InvalidFormat)?;
    let switch_count = parts[4]
      .trim()
      .parse::<u16>()
      .map_err(|_| FastResponseError::InvalidFormat)?;

    let board_revision = name
      .split('-')
      .last()
      .unwrap_or("0")
      .parse::<u16>()
      .unwrap_or(0);

    Ok(NodeInfo::Success {
      node_id,
      name,
      board_revision,
      firmware_version,
      driver_count,
      switch_count,
    })
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeInfo {
  Success {
    node_id: u8,
    name: String,
    board_revision: u16,
    firmware_version: String,
    driver_count: u16,
    switch_count: u16,
  },
  Failed,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_success() {
    let data = "02,FP-I/O-0804-1  ,00.89,04,08,04,06,00,00,00,00";
    let result = NodeNameCommand::new(2).parse(RawResponse {
      prefix: "NN:".to_string(),
      payload: data.to_string(),
      ..Default::default()
    });

    assert!(result.is_ok());
    match result.unwrap() {
      NodeInfo::Success {
        node_id,
        name,
        board_revision,
        firmware_version,
        driver_count,
        switch_count,
      } => {
        assert_eq!(node_id, 2);
        assert_eq!(name, "FP-I/O-0804-1");
        assert_eq!(board_revision, 1);
        assert_eq!(firmware_version, "00.89");
        assert_eq!(driver_count, 4);
        assert_eq!(switch_count, 8);
      }
      _ => panic!("Expected NodeInfo"),
    }
  }
}
