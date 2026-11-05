pub struct TextFormatting;

impl TextFormatting {
  /// Formats a number with commas as thousands separators.
  pub fn number(number: impl Into<i64>) -> String {
    let number = number.into();
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

  pub fn abbreviate_num(num: impl Into<f64>, max_len: usize) -> String {
    let mut num = num.into();
    let suffixes = ["", "k", "M", "G", "T", "P"];
    let mut suffix_idx = 0;

    // Scale down by 1000s
    while num >= 1000.0 && suffix_idx < suffixes.len() - 1 {
      num /= 1000.0;
      suffix_idx += 1;
    }

    let suffix = suffixes[suffix_idx];

    // Unabbreviated numbers (under 1000): drop decimals entirely
    if suffix_idx == 0 {
      return format!("{:.0}", num.floor());
    }

    // Try decimal precisions from 2 down to 0, truncating instead of rounding
    for decimals in (0..=2).rev() {
      let factor = 10.0_f64.powi(decimals as i32);
      let truncated = (num * factor).floor() / factor;

      let formatted = if decimals == 0 {
        format!("{:.0}{}", truncated, suffix)
      } else {
        format!("{:.1$}{2}", truncated, decimals, suffix)
      };

      if formatted.len() <= max_len {
        return formatted;
      }
    }

    // Fallback: zero decimal truncation
    format!("{:.0}{}", num.floor(), suffix)
  }
}
