use glam::{Affine2, Mat2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop, Shape};

/// Cap on the copies one `Revolve` makes, well past where they can be told apart
const MAX_COPIES: f32 = 360.0;

/// Moves, rotates, scales, copies, or cuts a node
///
/// `Mirror`, `Revolve`, and `Divide` are for shapes. Their `center` is in the space the transforms before them put the
/// shape in, and the transforms after them apply to everything they leave, so `[Revolve(..), Translate(..)]` moves
/// every copy. A sweep follows only the shape itself, and a fill ignores them.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Transform {
  Translate(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
  /// Degrees clockwise (y points down), around the node's center as placed by the transforms before this one.
  ///
  /// A `Sequence` has no second axis, so there it scrolls the drawing along the line with the ends wrapped together:
  /// 360 degrees is one full lap.
  Rotate(Prop<f32>),
  /// The same on both axes. A shape's own size, such as a `Rect`'s width and height, gives it a different one on each.
  Scale(Prop<f32>),
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
  Divide {
    #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
    center: Prop<Vec2>,
    rotation: Prop<f32>,
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
      Self::Scale(scale) => Affine2::from_scale(Vec2::splat(scale.resolve(env))),
      Self::Fit(traversal) => {
        let ends = env.visits.and_then(|visits| visits.ends(traversal)).filter(|(start, end)| start.distance(*end) > f32::EPSILON);
        let Some((start, end)) = ends else {
          return Affine2::IDENTITY;
        };
        let along = end - start;
        let fit = Affine2::from_scale_angle_translation(Vec2::splat(along.length()), along.to_angle(), (start + end) / 2.0);
        env.fill_frame * fit
      }
      // the shape itself stays where it is, and `copies` and `cut` say what's added to it or taken away
      Self::Mirror { .. } | Self::Revolve { .. } | Self::Divide { .. } => Affine2::IDENTITY,
    }
  }

  /// Where the node's center is after this step, given where it was and this step's affine. A `Mirror` or `Revolve`
  /// moves it to the middle of what it draws, so that a `Rotate` after one turns all of it together.
  pub(crate) fn center(&self, center: Vec2, affine: Affine2, env: &Env) -> Vec2 {
    match self {
      Self::Mirror { center: on_line, rotation } => {
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
        let reflect = Affine2::from_translation(center) * Affine2::from_mat2(flip) * Affine2::from_translation(-center);
        Some(vec![Affine2::IDENTITY, reflect])
      }
      Self::Revolve { center, angle, count } => {
        let center = center.resolve(env);
        let count = count.resolve_truncated(env).floor().clamp(0.0, MAX_COPIES) as u32;
        let step = angle.resolve(env).to_radians() / count.max(1) as f32;
        let turn = |copy: u32| {
          Affine2::from_translation(center) * Affine2::from_angle(step * copy as f32) * Affine2::from_translation(-center)
        };
        Some((0..count).map(turn).collect())
      }
      _ => None,
    }
  }

  /// The center of a `Mirror`, `Revolve`, or `Divide`
  pub(crate) fn copy_center(&self, env: &Env) -> Option<Vec2> {
    match self {
      Self::Mirror { center, .. } | Self::Revolve { center, .. } | Self::Divide { center, .. } => Some(center.resolve(env)),
      _ => None,
    }
  }

  /// For a `Divide`, the side of its line that's cut away
  pub(crate) fn cut(&self, env: &Env) -> Option<Cut> {
    match self {
      Self::Divide { center, rotation } => Some(Cut { point: center.resolve(env), out: line_direction(rotation, env).perp() }),
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
      Self::Rotate(prop) | Self::Scale(prop) => prop.is_animated(),
      Self::Fit(_) => true,
      Self::Mirror { center, rotation } | Self::Divide { center, rotation } => center.is_animated() || rotation.is_animated(),
      Self::Revolve { center, angle, count } => center.is_animated() || angle.is_animated() || count.is_animated(),
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
    Self { point: affine.transform_point2(self.point), out: out.normalize_or(self.out) }
  }
}

/// The direction of a `Mirror`'s or `Divide`'s line: straight down at 0 degrees, turning clockwise
fn line_direction(rotation: &Prop<f32>, env: &Env) -> Vec2 {
  Vec2::from_angle((90.0 + rotation.resolve(env)).to_radians())
}
