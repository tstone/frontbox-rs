use std::time::Duration;

pub fn request(
  switch_id: usize,
  reporting: SwitchReportingMode,
  debounce_close: Option<Duration>,
  debounce_open: Option<Duration>,
) -> String {
  // https://fastpinball.com/fast-serial-protocol/net/sl/
  format!(
    "SL:{:X},{},{:X},{:X}\r",
    switch_id,
    reporting as u8,
    debounce_close
      .unwrap_or(Duration::from_millis(2))
      .as_millis(),
    debounce_open
      .unwrap_or(Duration::from_millis(20))
      .as_millis()
  )
}

pub enum SwitchReportingMode {
  None = 0,
  ReportNormal = 1,
  ReportInverted = 2,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_request() {
    let result = request(10, SwitchReportingMode::ReportNormal, None, None);
    assert_eq!(result, "SL:A,1,2,14\r");
  }
}
