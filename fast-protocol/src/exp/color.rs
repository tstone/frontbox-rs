#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
  pub r: u8,
  pub g: u8,
  pub b: u8,
}

impl Color {
  pub fn rgb(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b }
  }

  pub fn mix(&self, other: &Self, t: f32) -> Self {
    let r = self.r + ((other.r as f32 - self.r as f32) * t) as u8;
    let g = self.g + ((other.g as f32 - self.g as f32) * t) as u8;
    let b = self.b + ((other.b as f32 - self.b as f32) * t) as u8;

    Self { r, g, b }
  }

  pub fn mix_all(colors: &[Color]) -> Self {
    if colors.is_empty() {
      return Self::black();
    } else if colors.len() == 1 {
      return colors[0];
    } else if colors.len() == 2 {
      return colors[0].mix(&colors[1], 0.5);
    }

    let n = colors.len() as f32;
    let r = (colors.iter().map(|c| c.r as f32).sum::<f32>() / n) as u8;
    let g = (colors.iter().map(|c| c.g as f32).sum::<f32>() / n) as u8;
    let b = (colors.iter().map(|c| c.b as f32).sum::<f32>() / n) as u8;

    Self { r, g, b }
  }

  pub fn to_hex(&self) -> String {
    format!("{:02X}{:02X}{:02X}", self.r, self.g, self.b)
  }

  pub fn black() -> Self {
    Self::rgb(0, 0, 0)
  }
}

impl Default for Color {
  fn default() -> Self {
    Self::black()
  }
}
