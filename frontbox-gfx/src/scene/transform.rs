use glam::{Affine2, Mat2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::noise::{MAX_OCTAVES, default_octaves, to_cells};
use crate::scene::{Contour, Env, Inputs, NoiseStyle, Prop, Shape};

/// Cap on the copies one `Revolve` makes, well past where they can be told apart
const MAX_COPIES: f32 = 360.0;

/// Moves, rotates, scales, copies, cuts, or reshapes a node
///
/// `Mirror`, `Revolve`, `Bisect`, `Shift`, `Bulge`, and `Round` are for shapes. Their `center` is in the space the
/// transforms before them put the shape in, and the transforms after them apply to everything they leave, so
/// `[Revolve(..), Translate(..)]` moves every copy. A sweep follows only the shape itself, and a fill ignores them.
/// `Warp` is for fills, and a shape ignores it.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees clockwise (y points down), around the node's center as placed by the transforms before this one.
  ///
  /// A `Sequence` has no second axis, so there it scrolls the drawing along the line with the ends wrapped together:
  /// 360 degrees is one full lap.
  Rotate(Prop<f32>),
  /// The same on both axes, around an anchor. A shape's own size, such as a `Rect`'s width and height, gives it a
  /// different one on each.
  Scale(Scaling),
  /// Lays the node along a traversal of the layer's graph, by name: `(-0.5, 0)` on the node it starts from and
  /// `(0.5, 0)` on the node it visits last, scaled evenly on both axes. A gradient then runs from start to end at
  /// whatever angle the traversal takes.
  ///
  /// Transforms before it are in that frame, where start to end is 1 long, so `[Translate(Fixed(-0.5, 0.0)), Fit("bolt")]`
  /// centers a radial gradient on the start. Changes nothing outside a layer with a graph, or while the traversal visits
  /// nothing or ends where it started.
  Fit(String),
  /// Draws the shape and its reflection across a line through `center`. At 0 degrees the line runs straight down, so
  /// the reflection is the shape flipped left to right, and it turns clockwise from there.
  Mirror {
    #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
    center: Prop<Vec2>,
    rotation: Prop<f32>,
  },
  /// Draws `count` copies of the shape turned around `center`, spread over `angle` degrees clockwise: 3 over 180 are 60
  /// apart. The first is the shape where it is. `count` is truncated to a whole number, and none draws nothing.
  Revolve {
    #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
    center: Prop<Vec2>,
    angle: Prop<f32>,
    count: Prop<f32>,
  },
  /// Keeps the part of the shape on one side of a line through `center`. At 0 degrees the line runs straight down and
  /// the right side is kept; it turns clockwise from there, so at 90 the bottom is kept.
  Bisect {
    #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
    center: Prop<Vec2>,
    rotation: Prop<f32>,
  },
  /// Slides the shape's middle along a line through its center, and stretches the half it moves away from and squashes
  /// the other to meet it, while the ends stay put. At 0 degrees the line runs straight down, turning clockwise from
  /// there. `amount` (-1 to 1) is how far the middle moves, as a share of the way to that end, so at 1 one half is
  /// squashed flat. The shape's outline is what moves, so text, which has none, isn't shifted. A shift after a
  /// `Mirror` or `Revolve` shifts the shape before it's copied.
  Shift {
    amount: Prop<f32>,
    rotation: Prop<f32>,
  },
  /// Curves each of the shape's straight edges, out when positive and in when negative, by that fraction of the
  /// edge's length at its middle. A line has no inside, so out is away from the side it bends around. Curves, such as
  /// a circle's edge or a `Round`'s corners, stay as they are, so a bulge goes before a round for the round to take in
  /// the corners it leaves. Text isn't bulged, as with a `Shift`.
  Bulge(Prop<f32>),
  /// Rounds each of the shape's corners into an arc of this radius, in the space the transforms before it put the
  /// shape in. Where an edge is too short for the arcs at both its ends, they share it and are tighter. A line's ends
  /// stay as they are, and text isn't rounded, as with a `Shift`.
  Round(Prop<f32>),
  /// Pushes each point of the fill a different way by smooth noise, so it ripples and wavers: wavy gradient bands, heat
  /// haze over an image, swirled noise. The transforms before it are bent by it, and the ones after move what it bent,
  /// so a `Rotate` before it turns the drawing and one after turns the ripples too. Its noise is sized like a
  /// [`FillNode::Noise`](crate::scene::FillNode::Noise)'s, and its `evolution` loops the same way.
  Warp {
    /// How far a point is pushed at most, as a share of the region's shorter side
    amount: Prop<f32>,
    /// How big the ripples are, as a share of the region's shorter side
    scale: Prop<f32>,
    /// Layers of finer ripples on top, from 1 to 8, as for noise
    #[serde(default = "default_octaves")]
    octaves: u32,
    evolution: Prop<f32>,
    #[serde(default)]
    seed: u32,
  },
}

impl Transform {
  /// The affine for this step, given where the transforms before it put the node's center
  pub(crate) fn affine(&self, center: Vec2, env: &Env) -> Affine2 {
    match self {
      Self::Translate(offset) => Affine2::from_translation(offset.resolve(env)),
      Self::Rotate(degrees) => match env.lap {
        Some(lap) => Affine2::from_translation(Vec2::new(lap * degrees.resolve(env) / 360.0, 0.0)),
        None => {
          let turn = Affine2::from_angle(degrees.resolve(env).to_radians());
          Affine2::from_translation(center) * turn * Affine2::from_translation(-center)
        }
      },
      Self::Scale(Scaling { amount, .. }) => {
        let anchor = self.anchor(center, env).unwrap_or(center);
        Affine2::from_translation(anchor)
          * Affine2::from_scale(Vec2::splat(amount.resolve(env)))
          * Affine2::from_translation(-anchor)
      }
      Self::Fit(traversal) => {
        let ends = env
          .visits
          .and_then(|visits| visits.ends(traversal))
          .filter(|(start, end)| start.distance(*end) > f32::EPSILON);
        let Some((start, end)) = ends else {
          return Affine2::IDENTITY;
        };
        let along = end - start;
        let fit = Affine2::from_scale_angle_translation(
          Vec2::splat(along.length()),
          along.to_angle(),
          (start + end) / 2.0,
        );
        env.fill_frame * fit
      }
      // the shape itself stays where it is, and `copies` and `cut` say what's added to it or taken away
      Self::Mirror { .. }
      | Self::Revolve { .. }
      | Self::Bisect { .. }
      | Self::Shift { .. }
      | Self::Bulge(_)
      | Self::Round(_)
      // it bends the fill between the affines on either side of it, as `warp` says
      | Self::Warp { .. } => Affine2::IDENTITY,
    }
  }

  /// Where the node's center is after this step, given where it was and this step's affine. A `Mirror` or `Revolve`
  /// moves it to the middle of what it draws, so that a `Rotate` after one turns all of it together.
  pub(crate) fn center(&self, center: Vec2, affine: Affine2, env: &Env) -> Vec2 {
    match self {
      Self::Mirror {
        center: on_line,
        rotation,
      } => {
        let (on_line, along) = (on_line.resolve(env), line_direction(rotation, env));
        on_line + along * along.dot(center - on_line)
      }
      Self::Revolve { center, .. } => center.resolve(env),
      _ => affine.transform_point2(center),
    }
  }

  /// For a `Mirror` or `Revolve`, where each copy it draws is taken to, starting with the shape where it is. `None` for
  /// other transforms, which draw the shape once.
  pub(crate) fn copies(&self, env: &Env) -> Option<Vec<Affine2>> {
    match self {
      Self::Mirror { center, rotation } => {
        let (center, along) = (center.resolve(env), line_direction(rotation, env));
        // what's along the line stays, and what's across it flips
        let flip = Mat2::from_cols(along * along.x * 2.0, along * along.y * 2.0) - Mat2::IDENTITY;
        let reflect = Affine2::from_translation(center)
          * Affine2::from_mat2(flip)
          * Affine2::from_translation(-center);
        Some(vec![Affine2::IDENTITY, reflect])
      }
      Self::Revolve {
        center,
        angle,
        count,
      } => {
        let center = center.resolve(env);
        let count = count.resolve_truncated(env).floor().clamp(0.0, MAX_COPIES) as u32;
        let step = angle.resolve(env).to_radians() / count.max(1) as f32;
        let turn = |copy: u32| {
          Affine2::from_translation(center)
            * Affine2::from_angle(step * copy as f32)
            * Affine2::from_translation(-center)
        };
        Some((0..count).map(turn).collect())
      }
      _ => None,
    }
  }

  /// The center of a `Mirror`, `Revolve`, or `Bisect`
  pub(crate) fn copy_center(&self, env: &Env) -> Option<Vec2> {
    match self {
      Self::Mirror { center, .. } | Self::Revolve { center, .. } | Self::Bisect { center, .. } => {
        Some(center.resolve(env))
      }
      _ => None,
    }
  }

  /// The point a `Scale` keeps still, given where the transforms before it put the node's center
  pub(crate) fn anchor(&self, center: Vec2, env: &Env) -> Option<Vec2> {
    match self {
      Self::Scale(Scaling { anchor, .. }) => Some(center + anchor.resolve(env)),
      _ => None,
    }
  }

  /// The direction of a `Mirror`'s or `Bisect`'s line through its center, unit length
  pub(crate) fn line(&self, env: &Env) -> Option<Vec2> {
    match self {
      Self::Mirror { rotation, .. } | Self::Bisect { rotation, .. } => {
        Some(line_direction(rotation, env))
      }
      _ => None,
    }
  }

  /// For a `Bisect`, the side of its line that's cut away
  pub(crate) fn cut(&self, env: &Env) -> Option<Cut> {
    match self {
      Self::Bisect { center, rotation } => Some(Cut {
        point: center.resolve(env),
        out: line_direction(rotation, env).perp(),
      }),
      _ => None,
    }
  }

  /// For a `Shift`, `Bulge`, or `Round`, how it reshapes the shape's outline
  pub(crate) fn reshape(&self, env: &Env) -> Option<Reshape> {
    match self {
      Self::Shift { amount, rotation } => Some(Reshape::Shift(Shift {
        along: line_direction(rotation, env),
        amount: amount.resolve(env).clamp(-1.0, 1.0),
      })),
      Self::Bulge(amount) => Some(Reshape::Bulge(amount.resolve(env))),
      Self::Round(radius) => Some(Reshape::Round(radius.resolve(env))),
      _ => None,
    }
  }

  /// For a `Warp`, how it bends a fill whose unit box is `unit_box` across in scene units
  pub(crate) fn warp(&self, unit_box: Vec2, env: &Env) -> Option<Warp> {
    match self {
      Self::Warp {
        amount,
        scale,
        octaves,
        evolution,
        seed,
      } => Some(Warp {
        to_cells: to_cells(unit_box, scale.resolve(env)),
        // the push is a share of the shorter side, so it's as far across as it is down
        push: amount.resolve(env) * unit_box.min_element()
          / unit_box.max(Vec2::splat(f32::EPSILON)),
        octaves: *octaves,
        evolution: evolution.resolve(env),
        seed: *seed,
      }),
      _ => None,
    }
  }

  /// The path of a `Sweep` translate, if this is one
  pub(crate) fn path(&self) -> Option<&Shape> {
    match self {
      Self::Translate(Prop::Sweep(sweep)) => Some(&sweep.path),
      _ => None,
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    match self {
      Self::Translate(prop) => prop.is_animated(),
      Self::Rotate(prop) | Self::Bulge(prop) | Self::Round(prop) => prop.is_animated(),
      Self::Scale(Scaling { amount, anchor }) => amount.is_animated() || anchor.is_animated(),
      Self::Fit(_) => true,
      Self::Mirror { center, rotation } | Self::Bisect { center, rotation } => {
        center.is_animated() || rotation.is_animated()
      }
      Self::Shift { amount, rotation } => amount.is_animated() || rotation.is_animated(),
      Self::Warp {
        amount,
        scale,
        evolution,
        ..
      } => [amount, scale, evolution]
        .iter()
        .any(|prop| prop.is_animated()),
      Self::Revolve {
        center,
        angle,
        count,
      } => center.is_animated() || angle.is_animated() || count.is_animated(),
    }
  }

  pub(crate) fn read_inputs(&self, inputs: &mut Inputs) {
    match self {
      Self::Translate(prop) => prop.read_inputs(inputs),
      Self::Rotate(prop) | Self::Bulge(prop) | Self::Round(prop) => prop.read_inputs(inputs),
      Self::Scale(Scaling { amount, anchor }) => {
        amount.read_inputs(inputs);
        anchor.read_inputs(inputs);
      }
      Self::Fit(_) => {}
      Self::Mirror { center, rotation } | Self::Bisect { center, rotation } => {
        center.read_inputs(inputs);
        rotation.read_inputs(inputs);
      }
      Self::Shift { amount, rotation } => [amount, rotation]
        .iter()
        .for_each(|prop| prop.read_inputs(inputs)),
      Self::Warp {
        amount,
        scale,
        evolution,
        ..
      } => [amount, scale, evolution]
        .iter()
        .for_each(|prop| prop.read_inputs(inputs)),
      Self::Revolve {
        center,
        angle,
        count,
      } => {
        center.read_inputs(inputs);
        [angle, count]
          .iter()
          .for_each(|prop| prop.read_inputs(inputs));
      }
    }
  }
}

/// How much a [`Transform::Scale`] scales by, and what stays put
///
/// In RON: `Scale(amount: Fixed(2.0), anchor: Fixed((-10.0, -5.0)))`, with `anchor` left out for the center. A file can
/// give only the amount, `Scale(Fixed(2.0))`, as files did before there was an anchor.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "ScalingFile")]
pub struct Scaling {
  pub amount: Prop<f32>,
  /// The point that stays put, from the node's center as placed by the transforms before this one, in the same space.
  /// The center by default.
  #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
  pub anchor: Prop<Vec2>,
}

impl From<f32> for Scaling {
  /// Scales by `amount` around the center
  fn from(amount: f32) -> Self {
    Self {
      amount: amount.into(),
      anchor: Vec2::ZERO.into(),
    }
  }
}

/// A [`Scaling`] as a file gives it: with an anchor, or only the amount
#[derive(Deserialize)]
#[serde(untagged)]
enum ScalingFile {
  Anchored {
    amount: Prop<f32>,
    #[serde(default)]
    anchor: Option<Prop<Vec2>>,
  },
  Amount(Prop<f32>),
}

impl From<ScalingFile> for Scaling {
  fn from(file: ScalingFile) -> Self {
    match file {
      ScalingFile::Anchored { amount, anchor } => Self {
        amount,
        anchor: anchor.unwrap_or(Prop::Fixed(Vec2::ZERO)),
      },
      ScalingFile::Amount(amount) => Self {
        amount,
        anchor: Prop::Fixed(Vec2::ZERO),
      },
    }
  }
}

/// A half-plane taken away from a shape: everything on the side of a line through `point` that `out` points to
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cut {
  point: Vec2,
  /// Unit length
  out: Vec2,
}

impl Cut {
  /// Signed distance from the line, positive on the side cut away
  pub(crate) fn distance(&self, p: Vec2) -> f32 {
    (p - self.point).dot(self.out)
  }

  /// The same cut, in the space `affine` takes this one's to
  pub(crate) fn transformed(&self, affine: Affine2) -> Self {
    let out = affine.matrix2.inverse().transpose() * self.out;
    Self {
      point: affine.transform_point2(self.point),
      out: out.normalize_or(self.out),
    }
  }
}

/// A `Warp`, resolved
#[derive(Debug, Clone, Copy)]
pub(crate) struct Warp {
  /// Takes a point in the unit box to noise cells
  to_cells: Vec2,
  /// The most a point moves along each axis, in the unit box
  push: Vec2,
  octaves: u32,
  evolution: f32,
  seed: u32,
}

impl Warp {
  /// Where `p`, in the unit box, is drawn from
  pub(crate) fn apply(&self, p: Vec2) -> Vec2 {
    let cells = p * self.to_cells;
    // a noise for each axis, -1 to 1. Seeds past the octaves the first one uses, so the two don't share any.
    let along =
      |seed: u32| NoiseStyle::Smooth.sample(cells, self.evolution, self.octaves, seed) * 2.0 - 1.0;
    p + Vec2::new(along(self.seed), along(self.seed.wrapping_add(MAX_OCTAVES))) * self.push
  }
}

/// A transform that reshapes a shape's outline, resolved
#[derive(Debug, Clone, Copy)]
pub(crate) enum Reshape {
  Shift(Shift),
  /// How far its straight edges curve out
  Bulge(f32),
  /// Its corners' radius
  Round(f32),
}

impl Reshape {
  /// `contour` reshaped, in the space the transforms before this one put it in, where the shape's center is `center`
  pub(crate) fn apply(&self, contour: &Contour, center: Vec2) -> Contour {
    match self {
      Self::Shift(shift) => shift.apply(contour, center),
      Self::Bulge(amount) => contour.bulged(*amount),
      Self::Round(radius) => contour.rounded(*radius),
    }
  }
}

/// A `Shift`, resolved
#[derive(Debug, Clone, Copy)]
pub(crate) struct Shift {
  /// Unit length
  along: Vec2,
  amount: f32,
}

impl Shift {
  /// The line through `center` that `points` are shifted along, from their back end to their front end
  pub(crate) fn line(&self, points: &[Vec2], center: Vec2) -> ShiftLine {
    let (back, forward) = points
      .iter()
      .map(|p| (*p - center).dot(self.along))
      .fold((0.0_f32, 0.0_f32), |(back, forward), t| {
        (back.min(t), forward.max(t))
      });
    let middle = self.amount * if self.amount > 0.0 { forward } else { -back };
    let at = |t: f32| center + self.along * t;
    ShiftLine {
      back: at(back),
      forward: at(forward),
      middle: at(middle),
    }
  }

  /// `contour` shifted along the line through `center`. An edge that crosses the middle gets a point there, so each
  /// part of it stays straight.
  pub(crate) fn apply(&self, contour: &Contour, center: Vec2) -> Contour {
    let along = |p: Vec2| (p - center).dot(self.along);
    let line = self.line(&contour.points(), center);
    let (back, forward, middle) = (along(line.back), along(line.forward), along(line.middle));
    // each half stretched or squashed evenly between its end and the middle
    let moved = |t: f32| match t < 0.0 {
      true if back < 0.0 => back + (t - back) * (middle - back) / -back,
      false if forward > 0.0 => middle + t * (forward - middle) / forward,
      _ => t,
    };
    contour.split_and_moved(along, |p| p + self.along * (moved(along(p)) - along(p)))
  }
}

/// Where a `Shift` moves a shape: the ends of the shape along its line, and where it moves the middle to, in the space
/// the transforms before it put the shape in
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShiftLine {
  pub back: Vec2,
  pub forward: Vec2,
  pub middle: Vec2,
}

/// The direction of a `Mirror`'s, `Bisect`'s, or `Shift`'s line: straight down at 0 degrees, turning clockwise
fn line_direction(rotation: &Prop<f32>, env: &Env) -> Vec2 {
  Vec2::from_angle((90.0 + rotation.resolve(env)).to_radians())
}
