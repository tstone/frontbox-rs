use glam::{Affine2, Mat2, Vec2};
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Prop, Shape};

/// Cap on the copies one `Revolve` makes, well past where they can be told apart
const MAX_COPIES: f32 = 360.0;

/// Moves, rotates, scales, copies, or cuts a node
///
/// `Mirror`, `Revolve`, `Divide`, and `Shift` are for shapes. Their `center` is in the space the transforms before them put the
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
  /// Slides the shape's middle along a line through its center, and stretches the half it moves away from and squashes
  /// the other to meet it, while the ends stay put. At 0 degrees the line runs straight down, turning clockwise from
  /// there. `amount` (-1 to 1) is how far the middle moves, as a share of the way to that end, so at 1 one half is
  /// squashed flat. The shape's outline is what moves, so text, which has none, isn't shifted. A shift after a
  /// `Mirror` or `Revolve` shifts the shape before it's copied.
  Shift { amount: Prop<f32>, rotation: Prop<f32> },
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
      Self::Mirror { .. } | Self::Revolve { .. } | Self::Divide { .. } | Self::Shift { .. } => Affine2::IDENTITY,
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

  /// For a `Shift`, its line and how far it moves the middle
  pub(crate) fn shift(&self, env: &Env) -> Option<Shift> {
    match self {
      Self::Shift { amount, rotation } => Some(Shift { along: line_direction(rotation, env), amount: amount.resolve(env).clamp(-1.0, 1.0) }),
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
      Self::Shift { amount, rotation } => amount.is_animated() || rotation.is_animated(),
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
    let (back, forward) = points.iter().map(|p| (*p - center).dot(self.along)).fold((0.0_f32, 0.0_f32), |(back, forward), t| (back.min(t), forward.max(t)));
    let middle = self.amount * if self.amount > 0.0 { forward } else { -back };
    let at = |t: f32| center + self.along * t;
    ShiftLine { back: at(back), forward: at(forward), middle: at(middle) }
  }

  /// `outline` shifted along the line through `center`, both in the space `before` takes the outline to. An edge that
  /// crosses the middle gets a point there, so each part of it stays straight.
  pub(crate) fn apply(&self, outline: &[Vec2], closed: bool, before: Affine2, center: Vec2) -> Vec<Vec2> {
    let placed: Vec<Vec2> = outline.iter().map(|p| before.transform_point2(*p)).collect();
    let along = |p: Vec2| (p - center).dot(self.along);
    let line = self.line(&placed, center);
    let (back, forward, middle) = (along(line.back), along(line.forward), along(line.middle));
    // each half stretched or squashed evenly between its end and the middle
    let moved = |t: f32| match t < 0.0 {
      true if back < 0.0 => back + (t - back) * (middle - back) / -back,
      false if forward > 0.0 => middle + t * (forward - middle) / forward,
      _ => t,
    };
    let edges = if closed { placed.len() } else { placed.len().saturating_sub(1) };
    let mut split = Vec::with_capacity(placed.len() * 2);
    for i in 0..placed.len() {
      let (from, to) = (placed[i], placed[(i + 1) % placed.len()]);
      split.push(from);
      let (from_t, to_t) = (along(from), along(to));
      if i < edges && from_t * to_t < 0.0 {
        split.push(from.lerp(to, from_t / (from_t - to_t)));
      }
    }
    let back_to_local = before.inverse();
    split.into_iter().map(|p| back_to_local.transform_point2(p + self.along * (moved(along(p)) - along(p)))).collect()
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

/// The direction of a `Mirror`'s, `Divide`'s, or `Shift`'s line: straight down at 0 degrees, turning clockwise
fn line_direction(rotation: &Prop<f32>, env: &Env) -> Vec2 {
  Vec2::from_angle((90.0 + rotation.resolve(env)).to_radians())
}
