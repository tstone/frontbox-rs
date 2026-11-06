use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::color::RgbaColor;
use crate::scene::{Env, Prop};

/// Changes to a paint's colors, applied in order to whatever its kind produces
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorAdjust {
  /// Turns the hue by degrees, keeping saturation, lightness, and alpha
  HueShift(Prop<f32>),
}

impl ColorAdjust {
  pub(crate) fn apply(&self, color: Rgba<u8>, env: &Env) -> Rgba<u8> {
    match self {
      Self::HueShift(degrees) => color.hue_shift(degrees.resolve(env)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::scene::value::tests::env;
  use crate::scene::{Key, Params, Track};

  #[test]
  fn hue_shift_turns_the_color() {
    let params = Params::new();
    let adjust = ColorAdjust::HueShift(120.0.into());
    assert_eq!(adjust.apply(Rgba([255, 0, 0, 128]), &env(0.0, &params)), Rgba([0, 255, 0, 128]));
  }

  #[test]
  fn hue_shift_can_be_animated() {
    let params = Params::new();
    let adjust = ColorAdjust::HueShift(Prop::Track(Track {
      keys: vec![Key::new(0.0, 0.0, Curve::Linear), Key::new(3.0, 360.0, Curve::Linear)],
      cycle: Cycle::Forever,
    }));
    let red = Rgba([255, 0, 0, 255]);
    let at = |time| adjust.apply(red, &env(time, &params));
    assert_eq!(at(0.0), red);
    assert_eq!(at(1.0), Rgba([0, 255, 0, 255]));
    assert_eq!(at(2.0), Rgba([0, 0, 255, 255]));
    assert_eq!(at(3.0), red);
  }
}
