use crate::prelude::*;

pub struct Gamma {
  pub value: f32, // 1.0 = unchanged, <1.0 brightens midtones, >1.0 darkens
  pub other: Box<dyn ColorSequence>,
}

impl ColorSequence for Gamma {
  fn render(&self, count: usize) -> Vec<Rgba<u8>> {
    self
      .other
      .render(count)
      .into_iter()
      .map(|Rgba([r, g, b, a])| {
        let correct = |c: u8| {
          let normalized = c as f32 / 255.0;
          (normalized.powf(self.value) * 255.0)
            .round()
            .clamp(0.0, 255.0) as u8
        };
        Rgba([correct(r), correct(g), correct(b), a])
      })
      .collect()
  }
}
