use image::{Pixel, Rgba};
use serde::{Deserialize, Serialize};

use crate::color::RgbaColor;
use crate::scene::{Env, Prop};

/// Changes to a fill's colors, applied in order to whatever its kind produces
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorAdjust {
  /// Turns the hue by degrees, keeping saturation, lightness, and alpha
  HueShift(Prop<f32>),
  /// Multiplies alpha: 0 is see-through, 1 leaves it alone
  Opacity(Prop<f32>),
  /// Multiplies red, green, and blue: 0 is black, 1 leaves them alone, and past that they run out at full. Black stays
  /// black, so a fade to it is a move to 0.
  Brightness(Prop<f32>),
  /// Multiplies how far red, green, and blue are from the middle of their range: 0 is all mid gray, 1 leaves them alone,
  /// and past that they run out at either end
  Contrast(Prop<f32>),
}

impl ColorAdjust {
  pub(crate) fn apply(&self, color: Rgba<u8>, env: &Env) -> Rgba<u8> {
    match self {
      Self::HueShift(degrees) => color.hue_shift(degrees.resolve(env)),
      Self::Opacity(amount) => {
        let mut color = color;
        color[3] = to_channel(color[3] as f32 * amount.resolve(env));
        color
      }
      Self::Brightness(amount) => {
        let amount = amount.resolve(env);
        color.map_without_alpha(|channel| to_channel(channel as f32 * amount))
      }
      Self::Contrast(amount) => {
        let amount = amount.resolve(env);
        color.map_without_alpha(|channel| to_channel((channel as f32 - MIDDLE) * amount + MIDDLE))
      }
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::HueShift(amount) | Self::Opacity(amount) | Self::Brightness(amount) | Self::Contrast(amount) => amount.is_animated(),
    }
  }
}

/// The middle of a channel's range
const MIDDLE: f32 = 255.0 / 2.0;

/// The nearest a channel can come to a value, as a cast stops at either end of its range
fn to_channel(value: f32) -> u8 {
  value.round() as u8
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

  fn applied(adjust: ColorAdjust, color: Rgba<u8>) -> Rgba<u8> {
    let params = Params::new();
    adjust.apply(color, &env(0.0, &params))
  }

  #[test]
  fn opacity_multiplies_alpha_and_nothing_else() {
    let color = Rgba([10, 20, 30, 200]);
    assert_eq!(applied(ColorAdjust::Opacity(0.5.into()), color), Rgba([10, 20, 30, 100]));
    assert_eq!(applied(ColorAdjust::Opacity(0.0.into()), color), Rgba([10, 20, 30, 0]));
    assert_eq!(applied(ColorAdjust::Opacity(1.0.into()), color), color);
  }

  #[test]
  fn brightness_multiplies_the_colors_and_keeps_alpha() {
    let color = Rgba([100, 200, 0, 128]);
    assert_eq!(applied(ColorAdjust::Brightness(0.5.into()), color), Rgba([50, 100, 0, 128]));
    assert_eq!(applied(ColorAdjust::Brightness(0.0.into()), color), Rgba([0, 0, 0, 128]));
    assert_eq!(applied(ColorAdjust::Brightness(1.0.into()), color), color);
    assert_eq!(applied(ColorAdjust::Brightness(2.0.into()), color), Rgba([200, 255, 0, 128]));
  }

  #[test]
  fn contrast_spreads_the_colors_from_the_middle_and_keeps_alpha() {
    let color = Rgba([100, 200, 0, 128]);
    assert_eq!(applied(ColorAdjust::Contrast(1.0.into()), color), color);
    assert_eq!(applied(ColorAdjust::Contrast(0.0.into()), color), Rgba([128, 128, 128, 128]));
    assert_eq!(applied(ColorAdjust::Contrast(0.5.into()), color), Rgba([114, 164, 64, 128]));
    assert_eq!(applied(ColorAdjust::Contrast(2.0.into()), color), Rgba([73, 255, 0, 128]));
  }

  #[test]
  fn an_amount_past_either_end_of_a_channel_stops_there() {
    // a curve that overshoots can take an amount below zero
    let color = Rgba([100, 200, 0, 128]);
    assert_eq!(applied(ColorAdjust::Opacity((-1.0).into()), color), Rgba([100, 200, 0, 0]));
    assert_eq!(applied(ColorAdjust::Opacity(3.0.into()), color), Rgba([100, 200, 0, 255]));
    assert_eq!(applied(ColorAdjust::Brightness((-1.0).into()), color), Rgba([0, 0, 0, 128]));
  }

  #[test]
  fn each_adjust_can_be_animated() {
    let moving = || Prop::Track(Track { keys: vec![Key::new(0.0, 0.0, Curve::Linear)], cycle: Cycle::Once });
    assert!(ColorAdjust::Opacity(moving()).is_animated());
    assert!(ColorAdjust::Brightness(moving()).is_animated());
    assert!(ColorAdjust::Contrast(moving()).is_animated());
    assert!(!ColorAdjust::Contrast(1.0.into()).is_animated());
  }
}
