pub struct TextFormatting;

impl TextFormatting {
  /// Formats a number with commas as thousands separators.
  pub fn number(number: i64) -> String {
    let mut num_str = number.abs().to_string();
    let mut formatted = String::new();

    while num_str.len() > 3 {
      let chunk = num_str.split_off(num_str.len() - 3);
      formatted = format!(",{}{}", chunk, formatted);
    }
    formatted = format!("{}{}", num_str, formatted);

    if number < 0 {
      formatted = format!("-{}", formatted);
    }
    formatted
  }
}
