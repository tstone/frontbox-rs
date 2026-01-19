pub fn request(platform_id: u16, switch_reporting: Option<SwitchReporting>) -> String {
  format!(
    "CH:{:04},{}\r",
    platform_id,
    switch_reporting.unwrap_or(SwitchReporting::None) as u8
  )
}

#[derive(Debug, Clone)]
pub enum SwitchReporting {
  None = 0,
  Verbose = 1,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = request(65, Some(SwitchReporting::Verbose));
    assert_eq!(result, "CH:0065,1\r");
  }
}
