use crate::protocol::{FastResponse, FastResponseError};

const ID_REQUEST: &[u8] = b"ID:\r";
pub fn request() -> &'static [u8] {
  ID_REQUEST
}

pub fn response(data: &str) -> Result<FastResponse, FastResponseError> {
  let parts: Vec<&str> = data.split(' ').filter(|part| !part.is_empty()).collect();
  if parts.len() != 3 {
    return Err(FastResponseError::InvalidFormat);
  }

  let processor = parts[0].trim().to_string();
  let product_number = parts[1].trim().to_string();
  let firmware_version = parts[2].trim().to_string();
  Ok(FastResponse::IdResponse {
    processor,
    product_number,
    firmware_version,
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_response_success() {
    let data = "FP-CPU-002  3208 2.00";
    let result = response(data);

    assert!(result.is_ok());
    match result.unwrap() {
      FastResponse::IdResponse {
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
