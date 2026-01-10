use crate::protocol::FastResponse;
use crate::serial::parser::FastResponseError;

const ID_REQUEST: &[u8] = b"ID:\r";
pub fn request() -> &'static [u8] {
  ID_REQUEST
}

pub fn response(data: &[u8]) -> Result<FastResponse, FastResponseError> {
  if data == b"F" {
    Ok(FastResponse::Failed("ID".to_string()))
  } else {
    let parts: Vec<&[u8]> = data.split(|&b| b == b' ').collect();
    if parts.len() != 3 {
      return Err(FastResponseError::InvalidFormat);
    }

    let processor = String::from_utf8_lossy(parts[0]).to_string();
    let product_number = String::from_utf8_lossy(parts[1]).to_string();
    let firmware_version = String::from_utf8_lossy(parts[2]).trim().to_string();
    Ok(FastResponse::IdResponse {
      processor,
      product_number,
      firmware_version,
    })
  }
}
