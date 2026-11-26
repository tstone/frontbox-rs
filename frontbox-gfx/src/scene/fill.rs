use std::collections::BTreeMap;
use std::sync::Arc;

use glam::{Affine2, Vec2};
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::animation::Lerp;
use crate::scene::noise::{default_octaves, to_cells};
use crate::scene::{
  Bend, CellStyle, Color, ColorAdjust, Env, Inputs, Node, NoiseStyle, Picture, Prop, Transform,
};

/// The colors a layer is drawn with
///
/// A fill is defined over a unit box from `-0.5` to `0.5`, which is stretched over its region's local bounds (or the
/// viewport, with no region). So a fill always fits its region, and moves and rotates with it. An image at its
/// [`ImageSize::Native`] size is the exception: each of its pixels is one scene unit, centered on the box.
pub type Fill = Node<FillNode, ColorAdjust>;

impl Fill {
  /// The fill as it is in `env`, to color any number of samples with. `footprint` is the samples' radius along each
  /// axis, in unit box units, and `size` is the unit box's size in scene units. `None` if a transform collapses it, so
  /// it draws nothing.
  pub(crate) fn prepare<'a>(
    &'a self,
    footprint: Vec2,
    size: Vec2,
    env: &Env<'a>,
  ) -> Option<PreparedFill<'a>> {
    // on a Sequence, the lap is the unit box plus one sample's width, as samples reach half a width past each end
    let env = &Env {
      lap: env.lap.map(|_| 1.0 + 2.0 * footprint.x),
      ..*env
    };
    let to_scene = self.to_scene(Vec2::ZERO, env)?;
    let (bends, to_local) = self.bends(size, env);
    // a `Rotate` on a `Sequence` repeats every lap, after everything else
    let lap = self.lap(env).map(|lap| {
      let around = Bend::Repeat {
        center: to_scene.transform_point2(Vec2::ZERO),
        size: Vec2::new(lap, 0.0),
      };
      (Affine2::IDENTITY, around)
    });
    Some(PreparedFill {
      bends: lap.into_iter().chain(bends).collect(),
      to_local,
      footprint: to_scene.inverse().matrix2.abs() * footprint,
      base: self.base.resolve(size, env),
      adjust: self
        .adjust
        .iter()
        .map(|adjust| (adjust, adjust.amount(env)))
        .collect(),
    })
  }

  /// The way back from a sample point to the fill's local space, split at each transform that bends it: the affine to
  /// take before each bend, starting from the last, and the affine after the first. Without one, that's just `to_scene`
  /// inverted.
  fn bends(&self, unit_box: Vec2, env: &Env) -> (Vec<(Affine2, Bend)>, Affine2) {
    let mut bends = Vec::new();
    // the transforms after the bend last met, combined
    let mut after = Affine2::IDENTITY;
    for (transform, (affine, center)) in
      self.transform.iter().zip(self.steps(Vec2::ZERO, env)).rev()
    {
      match transform.bend(center, unit_box, env) {
        Some(bend) => {
          bends.push((after.inverse(), bend));
          after = Affine2::IDENTITY;
        }
        None => after *= affine,
      }
    }
    (bends, after.inverse())
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.base.is_animated()
      || self.transform.iter().any(Transform::is_animated)
      || self.adjust.iter().any(ColorAdjust::is_animated)
  }

  pub(crate) fn read_inputs(&self, inputs: &mut Inputs) {
    self.base.read_inputs(inputs);
    self
      .transform
      .iter()
      .for_each(|transform| transform.read_inputs(inputs));
    self
      .adjust
      .iter()
      .for_each(|adjust| adjust.read_inputs(inputs));
  }
}

/// The base content of a fill
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FillNode {
  Solid(Prop<Color>),
  /// Runs left to right. Add a `Rotate` transform for other directions, or to move where a `Conical` one starts.
  Gradient {
    /// In any order, as they're drawn by where they are at the time
    stops: Vec<Stop>,
    #[serde(default)]
    style: GradientStyle,
  },
  /// Smooth random noise, its value from 0 to 1 colored by `stops` like a gradient's position. With stops from
  /// transparent to opaque, it makes a noise mask. Unlike a gradient it doesn't stretch with its region: its cells stay
  /// square, sized by the region's shorter side, and a `Scale` transform stretches them on purpose.
  Noise {
    #[serde(default, skip_serializing_if = "NoiseStyle::is_smooth")]
    #[cfg_attr(feature = "ts", ts(as = "Option<NoiseStyle>", optional))]
    style: NoiseStyle,
    /// How big the largest blobs are, as a share of the region's shorter side: 0.25 fits four across it
    scale: Prop<f32>,
    /// Layers of finer detail summed on top, each half the size and half as strong as the one before, from 1 (just the
    /// blobs) to 8. More takes longer to draw.
    #[serde(default = "default_octaves")]
    octaves: u32,
    /// Changes the noise in place, where a `Translate` scrolls it. It comes back to where it started at every whole
    /// number, so animating it from 0 to a whole number over a loop repeats seamlessly; to a larger one changes faster.
    evolution: Prop<f32>,
    /// Picks a different noise for the same settings
    #[serde(default)]
    seed: u32,
    /// In any order, as they're drawn by where they are at the time
    stops: Vec<Stop>,
  },
  /// Cells around scattered points, like cracked mud or a mosaic, their value from 0 to 1 colored by `stops` like
  /// noise's. Sized as noise is, by the region's shorter side.
  Cells {
    #[serde(default, skip_serializing_if = "CellStyle::is_edges")]
    #[cfg_attr(feature = "ts", ts(as = "Option<CellStyle>", optional))]
    style: CellStyle,
    /// How big a cell is on average, as a share of the region's shorter side: 0.25 fits about four across it
    scale: Prop<f32>,
    /// How far each cell's point strays from a grid, from 0 (a grid of squares) to 1 (fully random)
    jitter: Prop<f32>,
    /// Moves each cell's point around a small circle, so the cells shift and wobble. It comes back to where it started
    /// at every whole number, as noise's does.
    evolution: Prop<f32>,
    /// Picks different cells for the same settings
    #[serde(default)]
    seed: u32,
    /// In any order, as they're drawn by where they are at the time
    stops: Vec<Stop>,
  },
  /// An image over the unit box at its `size`, and transparent outside it. On a `Sequence`, samples run along its
  /// middle row.
  Image {
    /// The image's name in the scene's [`Assets`](crate::scene::Assets): for a game, its path within the content
    /// folder. Empty draws nothing, for an image not chosen yet.
    name: String,
    /// Frame index for an animated image, 0 if not set. Truncated to a whole number, so animating it from 0 to the frame
    /// count gives each frame equal time. Clamped to the last frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    frame: Option<Prop<f32>>,
    #[serde(default, skip_serializing_if = "ImageSize::is_stretch")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ImageSize>", optional))]
    size: ImageSize,
  },
  /// One sprite from an image laid out as a grid of equal sprites, drawn like an `Image`. Its `Native` size is the
  /// sprite's pixels.
  Spritesheet {
    /// The image's name in the scene's [`Assets`](crate::scene::Assets): for a game, its path within the content
    /// folder. Empty draws nothing, for an image not chosen yet.
    name: String,
    rows: u32,
    columns: u32,
    /// Which sprite, counting across each row from the top left, 0 if not set. Truncated to a whole number like an
    /// `Image`'s frame, and clamped to the last sprite.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    sprite: Option<Prop<f32>>,
    #[serde(default, skip_serializing_if = "ImageSize::is_stretch")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ImageSize>", optional))]
    size: ImageSize,
  },
  /// The image an [`ImageGenerator`](crate::scene::ImageGenerator) makes, in code outside the scene, by the name it
  /// has in the scene's [`Assets`](crate::scene::Assets), drawn like an `Image`. `settings` sets the generator's
  /// settings by name, and one left out is at its default. Its `Native` size is the pixels of the image the generator
  /// makes. With no generator by that name, it draws nothing.
  External {
    generator: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[cfg_attr(
      feature = "ts",
      ts(as = "Option<BTreeMap<String, Prop<f32>>>", optional)
    )]
    settings: BTreeMap<String, Prop<f32>>,
    #[serde(default, skip_serializing_if = "ImageSize::is_stretch")]
    #[cfg_attr(feature = "ts", ts(as = "Option<ImageSize>", optional))]
    size: ImageSize,
  },
}

/// A gradient color stop
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stop {
  /// Where it is along the gradient, from 0 to 1. Moving it can take it past others, as stops are drawn in order of
  /// where they are, and two in the same place make a hard edge, the one earlier in the list on the low side.
  pub at: Prop<f32>,
  pub color: Prop<Color>,
}

impl Stop {
  pub fn new(at: impl Into<Prop<f32>>, color: impl Into<Prop<Color>>) -> Self {
    Self {
      at: at.into(),
      color: color.into(),
    }
  }

  fn is_animated(&self) -> bool {
    self.at.is_animated() || self.color.is_animated()
  }

  fn read_inputs(&self, inputs: &mut Inputs) {
    self.at.read_inputs(inputs);
    self.color.read_inputs(inputs);
  }
}

/// How a gradient maps across the unit box
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GradientStyle {
  /// Left to right, holding the end colors past either end
  #[default]
  Linear,
  /// Out from the center, reaching 1 at the middle of each edge of the box and holding the last color past that
  Radial,
  /// Clockwise around the center, starting at 3 o'clock. 0 and 1 meet there, so there's a hard edge unless their colors
  /// match.
  Conical,
}

impl GradientStyle {
  /// Position along the gradient, 0 to 1, of a point in the unit box
  fn at(self, p: Vec2) -> f32 {
    match self {
      Self::Linear => (p.x + 0.5).clamp(0.0, 1.0),
      Self::Radial => (p.length() * 2.0).clamp(0.0, 1.0),
      // in f64, as f32 `atan2` is off by one step on some platforms (macOS gives just under pi for 9 o'clock)
      Self::Conical => {
        ((p.y as f64).atan2(p.x as f64) / std::f64::consts::TAU).rem_euclid(1.0) as f32
      }
    }
  }
}

/// How big an image is drawn
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageSize {
  /// Stretched over the unit box, so it fills its region
  #[default]
  Stretch,
  /// One scene unit for each pixel, centered on the unit box. Whatever is past the region's edges is cut off, so a
  /// `Translate` pans across an image bigger than its region.
  Native,
}

impl ImageSize {
  fn is_stretch(&self) -> bool {
    *self == Self::Stretch
  }

  /// What a point in the unit box is multiplied by to give the point in the image's own unit box, for a unit box
  /// `unit_box` across in scene units and an image `pixels` across
  fn scale(self, unit_box: Vec2, pixels: Vec2) -> Vec2 {
    match self {
      Self::Stretch => Vec2::ONE,
      Self::Native => unit_box / pixels.max(Vec2::ONE),
    }
  }
}

impl FillNode {
  /// An image, on its first frame, stretched over its region
  pub fn image(name: impl Into<String>) -> Self {
    Self::Image {
      name: name.into(),
      frame: None,
      size: ImageSize::Stretch,
    }
  }

  fn is_animated(&self) -> bool {
    match self {
      Self::Solid(color) => color.is_animated(),
      Self::Gradient { stops, .. } => stops.iter().any(Stop::is_animated),
      Self::Noise {
        scale,
        evolution,
        stops,
        ..
      } => scale.is_animated() || evolution.is_animated() || stops.iter().any(Stop::is_animated),
      Self::Cells {
        scale,
        jitter,
        evolution,
        stops,
        ..
      } => {
        scale.is_animated()
          || jitter.is_animated()
          || evolution.is_animated()
          || stops.iter().any(Stop::is_animated)
      }
      Self::Image { frame, .. } => frame.as_ref().is_some_and(Prop::is_animated),
      Self::Spritesheet { sprite, .. } => sprite.as_ref().is_some_and(Prop::is_animated),
      Self::External { settings, .. } => settings.values().any(Prop::is_animated),
    }
  }

  fn read_inputs(&self, inputs: &mut Inputs) {
    match self {
      Self::Solid(color) => color.read_inputs(inputs),
      Self::Gradient { stops, .. } => stops.iter().for_each(|stop| stop.read_inputs(inputs)),
      Self::Noise {
        scale,
        evolution,
        stops,
        ..
      } => {
        scale.read_inputs(inputs);
        evolution.read_inputs(inputs);
        stops.iter().for_each(|stop| stop.read_inputs(inputs));
      }
      Self::Cells {
        scale,
        jitter,
        evolution,
        stops,
        ..
      } => {
        scale.read_inputs(inputs);
        jitter.read_inputs(inputs);
        evolution.read_inputs(inputs);
        stops.iter().for_each(|stop| stop.read_inputs(inputs));
      }
      Self::Image { frame, .. } => frame.iter().for_each(|frame| frame.read_inputs(inputs)),
      Self::Spritesheet { sprite, .. } => {
        sprite.iter().for_each(|sprite| sprite.read_inputs(inputs))
      }
      Self::External { settings, .. } => settings
        .values()
        .for_each(|setting| setting.read_inputs(inputs)),
    }
  }

  pub(crate) fn image_name(&self) -> Option<&str> {
    match self {
      Self::Image { name, .. } | Self::Spritesheet { name, .. } if !name.is_empty() => Some(name),
      _ => None,
    }
  }

  /// The name of the image generator that makes it, for an external fill
  pub fn generator_name(&self) -> Option<&str> {
    match self {
      Self::External { generator, .. } => Some(generator),
      _ => None,
    }
  }

  /// The names of the generator settings an external fill sets
  pub fn generator_settings(&self) -> impl Iterator<Item = &str> {
    let settings = match self {
      Self::External { settings, .. } => Some(settings.keys().map(String::as_str)),
      _ => None,
    };
    settings.into_iter().flatten()
  }

  /// The node resolved in `env`, for a unit box `unit_box` across in scene units
  fn resolve<'a>(&self, unit_box: Vec2, env: &Env<'a>) -> ResolvedFillNode<'a> {
    match self {
      Self::Solid(color) => ResolvedFillNode::Solid(color.resolve(env).0),
      Self::Gradient { stops, style } => ResolvedFillNode::Gradient {
        stops: resolve_stops(stops, env),
        style: *style,
      },
      Self::Noise {
        style,
        scale,
        octaves,
        evolution,
        seed,
        stops,
      } => ResolvedFillNode::Noise {
        style: *style,
        to_cells: to_cells(unit_box, scale.resolve(env)),
        octaves: *octaves,
        evolution: evolution.resolve(env),
        seed: *seed,
        stops: resolve_stops(stops, env),
      },
      Self::Cells {
        style,
        scale,
        jitter,
        evolution,
        seed,
        stops,
      } => ResolvedFillNode::Cells {
        style: *style,
        to_cells: to_cells(unit_box, scale.resolve(env)),
        jitter: jitter.resolve(env),
        evolution: evolution.resolve(env),
        seed: *seed,
        stops: resolve_stops(stops, env),
      },
      Self::Image { name, frame, size } => {
        let picture = env.assets.picture(name);
        ResolvedFillNode::Image {
          picture,
          frame: frame
            .as_ref()
            .map_or(0.0, |frame| frame.resolve_truncated(env)),
          scale: size.scale(unit_box, picture.map_or(Vec2::ONE, pixels)),
        }
      }
      Self::Spritesheet {
        name,
        rows,
        columns,
        sprite,
        size: image_size,
      } => {
        let (rows, columns) = ((*rows).max(1), (*columns).max(1));
        let sprite = sprite
          .as_ref()
          .map_or(0.0, |sprite| sprite.resolve_truncated(env))
          .max(0.0) as u32;
        let sprite = sprite.min(rows * columns - 1);
        let size = Vec2::new(1.0 / columns as f32, 1.0 / rows as f32);
        let min = Vec2::new((sprite % columns) as f32, (sprite / columns) as f32) * size;
        let picture = env.assets.picture(name);
        let sprite_pixels = picture.map_or(Vec2::ONE, |picture| pixels(picture) * size);
        ResolvedFillNode::Sprite {
          picture,
          min,
          size,
          scale: image_size.scale(unit_box, sprite_pixels),
        }
      }
      Self::External {
        generator,
        settings,
        size,
      } => {
        let picture = env.assets.image_generator(generator).map(|loaded| {
          loaded.generate(env.time, env.cycle, env.playback_seed, |name| {
            settings.get(name).map(|setting| setting.resolve(env))
          })
        });
        let pixels = picture.as_deref().map_or(Vec2::ONE, pixels);
        ResolvedFillNode::Generated {
          picture,
          scale: size.scale(unit_box, pixels),
        }
      }
    }
  }
}

/// A [`Fill`] resolved for one frame, with what coloring a sample needs
pub(crate) struct PreparedFill<'a> {
  /// From the sample point, each bend from the last, with the affine taken before it
  bends: Vec<(Affine2, Bend)>,
  /// To the fill's local space, after the bends
  to_local: Affine2,
  /// The samples' radius along each axis, in the fill's local space
  footprint: Vec2,
  base: ResolvedFillNode<'a>,
  /// Each adjust, with how much it adjusts by
  adjust: Vec<(&'a ColorAdjust, f32)>,
}

impl PreparedFill<'_> {
  /// The color at a point in the unit box
  pub(crate) fn color(&self, p: Vec2) -> Rgba<u8> {
    let p = self.bends.iter().fold(p, |p, (affine, bend)| {
      bend.apply(affine.transform_point2(p))
    });
    let color = self
      .base
      .color(self.to_local.transform_point2(p), self.footprint);
    self.adjust.iter().fold(color, |color, (adjust, amount)| {
      adjust.apply(color, *amount)
    })
  }
}

/// A [`FillNode`] with its props resolved
enum ResolvedFillNode<'a> {
  Solid(Rgba<u8>),
  /// Each stop's `at` and color
  Gradient {
    stops: Vec<(f32, Rgba<u8>)>,
    style: GradientStyle,
  },
  /// `to_cells` takes a point in the unit box to noise cells, where a cell is 1 across. Stops are as for `Gradient`.
  Noise {
    style: NoiseStyle,
    to_cells: Vec2,
    octaves: u32,
    evolution: f32,
    seed: u32,
    stops: Vec<(f32, Rgba<u8>)>,
  },
  /// As for `Noise`
  Cells {
    style: CellStyle,
    to_cells: Vec2,
    jitter: f32,
    evolution: f32,
    seed: u32,
    stops: Vec<(f32, Rgba<u8>)>,
  },
  /// `None` for an image that isn't loaded, which draws nothing. `scale` takes a point in the unit box to the image's
  /// own unit box.
  Image {
    picture: Option<&'a Picture>,
    frame: f32,
    scale: Vec2,
  },
  /// As for `Image`, for the image a generator made for this frame. `None` for a generator the assets don't have.
  Generated {
    picture: Option<Arc<Picture>>,
    scale: Vec2,
  },
  /// The part of an image from `min`, `size` across, in units of the whole image. `scale` is as for `Image`, to the
  /// sprite's own unit box.
  Sprite {
    picture: Option<&'a Picture>,
    min: Vec2,
    size: Vec2,
    scale: Vec2,
  },
}

impl ResolvedFillNode<'_> {
  fn color(&self, p: Vec2, footprint: Vec2) -> Rgba<u8> {
    match self {
      Self::Solid(color) => *color,
      Self::Gradient { stops, style } => gradient(stops, style.at(p)),
      Self::Noise {
        style,
        to_cells,
        octaves,
        evolution,
        seed,
        stops,
      } => gradient(
        stops,
        style.sample(p * *to_cells, *evolution, *octaves, *seed),
      ),
      Self::Cells {
        style,
        to_cells,
        jitter,
        evolution,
        seed,
        stops,
      } => gradient(
        stops,
        style.sample(p * *to_cells, *jitter, *evolution, *seed),
      ),
      Self::Image {
        picture,
        frame,
        scale,
      } => picture.map_or(Rgba([0, 0, 0, 0]), |picture| {
        picture.color(*frame, p * *scale, footprint * *scale)
      }),
      Self::Generated { picture, scale } => {
        picture.as_ref().map_or(Rgba([0, 0, 0, 0]), |picture| {
          picture.color(0.0, p * *scale, footprint * *scale)
        })
      }
      Self::Sprite {
        picture,
        min,
        size,
        scale,
      } => picture.map_or(Rgba([0, 0, 0, 0]), |picture| {
        picture.color_within(0.0, *min, *size, p * *scale, footprint * *scale)
      }),
    }
  }
}

/// A picture's width and height in pixels
fn pixels(picture: &Picture) -> Vec2 {
  Vec2::new(picture.width() as f32, picture.height() as f32)
}

/// Each stop's position and color in `env`, in order of position. A stable sort, so stops in the same place stay in
/// list order.
fn resolve_stops(stops: &[Stop], env: &Env) -> Vec<(f32, Rgba<u8>)> {
  let mut resolved: Vec<_> = stops
    .iter()
    .map(|stop| (stop.at.resolve(env), stop.color.resolve(env).0))
    .collect();
  resolved.sort_by(|a, b| a.0.total_cmp(&b.0));
  resolved
}

fn gradient(stops: &[(f32, Rgba<u8>)], t: f32) -> Rgba<u8> {
  let (Some(first), Some(last)) = (stops.first(), stops.last()) else {
    return Rgba([0, 0, 0, 0]);
  };
  if t <= first.0 {
    return first.1;
  }

  for pair in stops.windows(2) {
    let ((a_at, a), (b_at, b)) = (pair[0], pair[1]);
    if t <= b_at {
      let span = b_at - a_at;
      let local = if span > 0.0 { (t - a_at) / span } else { 1.0 };
      return a.interpolate(&b, local);
    }
  }

  last.1
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::scene::Transform;
  use crate::scene::value::tests::env;
  use crate::scene::{Key, Keyframes, Prop, Variables};

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

  fn red_to_blue() -> Fill {
    styled(GradientStyle::Linear)
  }

  fn styled(style: GradientStyle) -> Fill {
    Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
      style,
    })
  }

  fn color(fill: &Fill, x: f32, y: f32) -> Rgba<u8> {
    let variables = Variables::new();
    fill
      .prepare(Vec2::ZERO, Vec2::ONE, &env(0.0, &variables))
      .map_or(Rgba([0, 0, 0, 0]), |fill| fill.color(Vec2::new(x, y)))
  }

  #[test]
  fn gradient_runs_left_to_right_and_clamps() {
    let fill = red_to_blue();
    assert_eq!(color(&fill, -0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
    assert_eq!(color(&fill, -3.0, 0.0), RED);
    assert_eq!(color(&fill, 3.0, 0.0), BLUE);
    assert_eq!(color(&fill, 0.0, 0.0), Rgba([127, 0, 128, 255]));
  }

  #[test]
  fn a_radial_gradient_runs_out_from_the_middle() {
    let fill = styled(GradientStyle::Radial);
    assert_eq!(color(&fill, 0.0, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
    assert_eq!(color(&fill, 0.0, -0.5), BLUE);
    assert_eq!(color(&fill, 0.25, 0.0), color(&fill, 0.0, -0.25));
    assert_eq!(color(&fill, 0.25, 0.0), Rgba([127, 0, 128, 255]));
    assert_eq!(color(&fill, 0.5, 0.5), BLUE);
  }

  #[test]
  fn a_conical_gradient_runs_clockwise_from_the_right() {
    let fill = styled(GradientStyle::Conical);
    assert_eq!(color(&fill, 0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.0, 0.5), Rgba([191, 0, 64, 255]));
    assert_eq!(color(&fill, -0.5, 0.0), Rgba([127, 0, 128, 255]));
    assert_eq!(color(&fill, 0.0, -0.5), Rgba([64, 0, 191, 255]));
    assert_eq!(color(&fill, 0.5, -0.0001), BLUE);
  }

  #[test]
  fn rotate_turns_where_a_conical_gradient_starts() {
    let fill = styled(GradientStyle::Conical).transform(Transform::Rotate(90.0.into()));
    // just past the start, as exactly on it could land either side of where the ends meet
    assert_eq!(color(&fill, -0.0001, 0.5), RED);
  }

  #[test]
  fn rotate_makes_a_gradient_vertical() {
    let fill = red_to_blue().transform(Transform::Rotate(90.0.into()));
    assert_eq!(color(&fill, 0.5, -0.5), RED);
    assert_eq!(color(&fill, -0.5, 0.5), BLUE);
  }

  #[test]
  fn a_fill_moved_then_rotated_turns_around_its_own_middle() {
    let fill = red_to_blue()
      .transform(Transform::Translate(Vec2::new(1.0, 0.0).into()))
      .transform(Transform::Rotate(90.0.into()));
    assert_eq!(color(&fill, 1.0, -0.5), RED);
    assert_eq!(color(&fill, 1.0, 0.5), BLUE);
  }

  #[test]
  fn a_fill_ignores_transforms_that_copy_or_cut_a_shape() {
    let center = || Prop::Fixed(Vec2::ZERO);
    let fill = red_to_blue()
      .transform(Transform::Mirror {
        center: center(),
        rotation: 0.0.into(),
      })
      .transform(Transform::Revolve {
        center: center(),
        angle: 180.0.into(),
        count: 3.0.into(),
      })
      .transform(Transform::Bisect {
        center: center(),
        rotation: 0.0.into(),
      });
    assert_eq!(color(&fill, -0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
  }

  #[test]
  fn stops_in_the_middle() {
    let fill = Fill::new(FillNode::Gradient {
      stops: vec![
        Stop::new(0.25, Color(RED)),
        Stop::new(0.5, Color(Rgba([0, 255, 0, 255]))),
        Stop::new(0.75, Color(BLUE)),
      ],
      style: GradientStyle::Linear,
    });
    assert_eq!(color(&fill, -0.4, 0.0), RED);
    assert_eq!(color(&fill, 0.0, 0.0), Rgba([0, 255, 0, 255]));
    assert_eq!(color(&fill, 0.4, 0.0), BLUE);
  }

  #[test]
  fn stops_are_drawn_in_order_of_where_they_are() {
    let fill = Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(1.0, Color(BLUE)), Stop::new(0.0, Color(RED))],
      style: GradientStyle::Linear,
    });
    assert_eq!(color(&fill, -0.5, 0.0), RED);
    assert_eq!(color(&fill, 0.5, 0.0), BLUE);
  }

  #[test]
  fn two_stops_in_one_place_make_a_hard_edge() {
    let green = Rgba([0, 255, 0, 255]);
    let fill = Fill::new(FillNode::Gradient {
      stops: vec![Stop::new(0.5, Color(RED)), Stop::new(0.5, Color(green))],
      style: GradientStyle::Linear,
    });
    assert_eq!(color(&fill, -0.01, 0.0), RED);
    assert_eq!(color(&fill, 0.01, 0.0), green);
  }

  #[test]
  fn a_stop_can_move_over_time() {
    let variables = Variables::new();
    let moving = Prop::Keyframes(Keyframes {
      keys: vec![
        Key::new(0.0, 0.0, Curve::Linear),
        Key::new(1.0, 1.0, Curve::Linear),
      ],
      cycle: Cycle::Once,
    });
    // a hard edge from red to blue that sweeps left to right
    let fill = Fill::new(FillNode::Gradient {
      stops: vec![
        Stop::new(moving.clone(), Color(RED)),
        Stop::new(moving, Color(BLUE)),
      ],
      style: GradientStyle::Linear,
    });
    assert!(fill.is_animated());
    let at = |time, x| {
      fill
        .prepare(Vec2::ZERO, Vec2::ONE, &env(time, &variables))
        .map(|fill| fill.color(Vec2::new(x, 0.0)))
    };
    assert_eq!(at(0.25, -0.3), Some(RED));
    assert_eq!(at(0.25, -0.2), Some(BLUE));
    assert_eq!(at(0.75, 0.2), Some(RED));
    assert_eq!(at(0.75, 0.3), Some(BLUE));
  }

  #[test]
  fn adjusts_apply_in_order_after_the_kind() {
    let mut fill = red_to_blue();
    fill.adjust.push(ColorAdjust::HueShift(120.0.into()));
    fill.adjust.push(ColorAdjust::HueShift((-60.0).into()));
    // red turned 60 degrees is yellow; blue turned 60 degrees is magenta
    assert_eq!(color(&fill, -0.5, 0.0), Rgba([255, 255, 0, 255]));
    assert_eq!(color(&fill, 0.5, 0.0), Rgba([255, 0, 255, 255]));
  }

  fn noise(stops: Vec<Stop>) -> Fill {
    Fill::new(FillNode::Noise {
      style: NoiseStyle::Smooth,
      scale: 0.25.into(),
      octaves: 3,
      evolution: 0.0.into(),
      seed: 0,
      stops,
    })
  }

  fn color_sized(fill: &Fill, size: Vec2, p: Vec2) -> Rgba<u8> {
    let variables = Variables::new();
    fill
      .prepare(Vec2::ZERO, size, &env(0.0, &variables))
      .map_or(Rgba([0, 0, 0, 0]), |fill| fill.color(p))
  }

  #[test]
  fn noise_is_colored_by_its_stops() {
    let fill = noise(vec![
      Stop::new(0.0, Color(Rgba([255, 255, 255, 0]))),
      Stop::new(1.0, Color(Rgba([255, 255, 255, 255]))),
    ]);
    let alphas: Vec<u8> = (0..50)
      .map(|i| color(&fill, i as f32 / 50.0 - 0.5, 0.1)[3])
      .collect();
    assert!(
      alphas.iter().min().unwrap() < &100 && alphas.iter().max().unwrap() > &155,
      "{alphas:?}"
    );
  }

  #[test]
  fn noise_cells_stay_square_on_a_wide_region() {
    let fill = red_noise();
    // a quarter of the way across a 4:1 region is as far, in scene units, as all the way across a square one's height
    for (x, y) in [(0.1, 0.2), (-0.05, -0.3), (0.12, 0.0)] {
      assert_eq!(
        color_sized(&fill, Vec2::new(4.0, 1.0), Vec2::new(x, y)),
        color_sized(&fill, Vec2::ONE, Vec2::new(x * 4.0, y)),
      );
    }
  }

  #[test]
  fn noise_is_sized_by_the_regions_shorter_side() {
    let fill = red_noise();
    for (x, y) in [(0.1, 0.2), (-0.05, -0.3)] {
      assert_eq!(
        color_sized(&fill, Vec2::new(10.0, 10.0), Vec2::new(x, y)),
        color_sized(&fill, Vec2::new(30.0, 30.0), Vec2::new(x, y)),
      );
    }
  }

  #[test]
  fn cells_without_jitter_are_a_grid_colored_by_their_stops() {
    let fill = Fill::new(FillNode::Cells {
      style: CellStyle::Edges,
      scale: 0.5.into(),
      jitter: 0.0.into(),
      evolution: 0.0.into(),
      seed: 0,
      stops: vec![Stop::new(0.0, Color(RED)), Stop::new(1.0, Color(BLUE))],
    });
    // two cells across a square region, so they meet down its middle and the middle of each is furthest from an edge
    assert_eq!(color(&fill, 0.0, 0.25), BLUE);
    assert_eq!(color(&fill, 0.25, 0.25), RED);
  }

  fn red_noise() -> Fill {
    noise(vec![
      Stop::new(0.0, Color(RED)),
      Stop::new(1.0, Color(BLUE)),
    ])
  }

  fn warp(amount: f32) -> Transform {
    Transform::Warp {
      amount: amount.into(),
      scale: 0.3.into(),
      octaves: 2,
      evolution: 0.0.into(),
      seed: 0,
    }
  }

  fn warped(transform: Vec<Transform>) -> Fill {
    Fill {
      transform,
      ..red_to_blue()
    }
  }

  fn points() -> impl Iterator<Item = (f32, f32)> {
    (0..12).flat_map(|x| (0..12).map(move |y| (x as f32 / 20.0 - 0.3, y as f32 / 20.0 - 0.3)))
  }

  #[test]
  fn a_warp_pushes_a_gradient_around_by_no_more_than_its_amount() {
    let fill = warped(vec![warp(0.1)]);
    let pushes: Vec<f32> = points()
      .map(|(x, y)| {
        // the gradient runs red to blue left to right, so its blue says where across it was drawn from
        let from = color(&fill, x, y)[2] as f32 / 255.0 - 0.5;
        (from - x).abs()
      })
      .collect();
    assert!(pushes.iter().all(|push| *push <= 0.11), "{pushes:?}");
    assert!(pushes.iter().any(|push| *push > 0.03), "{pushes:?}");
  }

  #[test]
  fn a_warp_of_nothing_changes_nothing() {
    let (plain, fill) = (red_to_blue(), warped(vec![warp(0.0)]));
    for (x, y) in points() {
      assert_eq!(color(&fill, x, y), color(&plain, x, y));
    }
  }

  #[test]
  fn transforms_after_a_warp_move_its_ripples_and_ones_before_it_dont() {
    let down = || Transform::Translate(Vec2::new(0.0, 0.2).into());
    let alone = warped(vec![warp(0.1)]);
    // the gradient doesn't change going down, so moving it down first leaves only the ripples, where they were
    let before = warped(vec![down(), warp(0.1)]);
    let after = warped(vec![warp(0.1), down()]);
    for (x, y) in points() {
      assert_eq!(color(&before, x, y), color(&alone, x, y));
      assert_eq!(color(&after, x, y + 0.2), color(&alone, x, y));
    }
    assert!(points().any(|(x, y)| color(&after, x, y) != color(&alone, x, y)));
  }

  #[test]
  fn solid_ignores_position() {
    let fill = Fill::new(FillNode::Solid(Color(RED).into()));
    assert_eq!(color(&fill, 100.0, -3.0), RED);
  }
}
