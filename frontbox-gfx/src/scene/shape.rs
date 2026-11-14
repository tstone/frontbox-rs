use glam::{Affine2, Mat2, Vec2};
use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::scene::{
  Bounds, Cut, Env, Node, PreparedStroke, Prop, Sample, ShapeAdjust, TextAlign, ShiftLine, TextOutline, TextSource, Transform, VerticalAlign, Wrap, layout_text,
};

/// Points in the outline polygon of a round shape
const ROUND_OUTLINE_POINTS: usize = 64;

/// Cap on a star's legs, well past where they can be told apart
const MAX_LEGS: f32 = 360.0;

/// Cap on the copies a shape's `Mirror`s and `Revolve`s make between them
const MAX_COPIES: usize = 1024;

/// Line segments per bezier span when flattening it to a polyline
const BEZIER_STEPS: usize = 24;

/// Line segments per bowed side of a star or diamond
const BOW_STEPS: usize = 16;

/// The area a layer is drawn in
pub type Shape = Node<ShapeNode, ShapeAdjust>;

/// Base shapes, defined around the origin in local space. Transforms place them in the scene.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShapeNode {
  Circle { radius: Prop<f32> },
  Ellipse {
    #[serde(alias = "r2")]
    rx: Prop<f32>,
    #[serde(alias = "r1")]
    ry: Prop<f32>,
  },
  Rect { width: Prop<f32>, height: Prop<f32> },
  /// Four corners: one at the top and one at the bottom, `height` apart, and one either side, `width` apart, halfway
  /// down. A `Shift` moves the side corners up or down. `bow` curves
  /// all four sides, out when positive and in when negative, by that fraction of a side's length at its middle.
  Diamond { width: Prop<f32>, height: Prop<f32>, bow: Prop<f32> },
  /// `legs` triangles out from the origin to `radius`, spaced evenly around it, each `leg_width` times `radius` across
  /// where its base crosses the origin, so the legs keep their shape as the radius changes. The first points up and
  /// they go clockwise, so one leg is a triangle with its base on the origin. `legs` is truncated to a whole number.
  /// `bow` curves the legs' sides, as a [`ShapeNode::Diamond`]'s, and leaves one leg's base straight.
  Star { radius: Prop<f32>, legs: Prop<f32>, leg_width: Prop<f32>, bow: Prop<f32> },
  /// A triangle pointing up with a smaller one cut out of its base: `width` across, `height` from the bottom corners to
  /// the tip, and the cut reaching `notch` (0 to 1) of the way up.
  Chevron { width: Prop<f32>, height: Prop<f32>, notch: Prop<f32> },
  /// A closed polygon
  Polygon {
    #[cfg_attr(feature = "ts", ts(as = "Vec<Prop<[f32; 2]>>"))]
    points: Vec<Prop<Vec2>>,
  },
  /// An open polyline. It has no inside, so it draws nothing as a layer's shape; it's only for a [`Sweep`] to follow.
  ///
  /// [`Sweep`]: crate::scene::Sweep
  #[serde(alias = "Route")]
  Polyline {
    #[cfg_attr(feature = "ts", ts(as = "Vec<Prop<[f32; 2]>>"))]
    points: Vec<Prop<Vec2>>,
  },
  /// An open, smooth curve through points. Like [`ShapeNode::Polyline`], it's only for a sweep to follow.
  Bezier { points: Vec<BezierPoint> },
  /// `text` in the font named `font`, in a box `width` by `height` around the origin. `size` is the font's em. Lines
  /// break at line breaks, and between words where the next would run past the box. `align` places each line across
  /// the box and `vertical_align` places them all up and down it, and what runs past the box is cut off. The box is
  /// what a fill is stretched over. With no font by that name, it draws nothing.
  Text {
    text: TextSource,
    font: String,
    size: Prop<f32>,
    width: Prop<f32>,
    height: Prop<f32>,
    #[serde(default)]
    align: TextAlign,
    #[serde(default)]
    vertical_align: VerticalAlign,
  },
}

/// A point a [`ShapeNode::Bezier`] passes through
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BezierPoint {
  #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
  pub point: Prop<Vec2>,
  /// Tangent handle, relative to `point`. The curve leaves toward `point + handle` and arrives from `point - handle`, so
  /// it's always smooth through the point.
  #[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))]
  pub handle: Prop<Vec2>,
}

/// A snapshot of a shape at one moment, with what an editor needs to draw it and its handles
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePose {
  /// The affine for each transform, in order
  pub transforms: Vec<Affine2>,
  /// Bounds in local space, before transforms
  pub bounds: Bounds,
  /// Local-space points of a polygon, polyline, or bezier. Empty for other shapes.
  pub points: Vec<Vec2>,
  /// Bezier handles, relative to their points. Empty for other shapes.
  pub handles: Vec<Vec2>,
  /// For each `Shift`, where it moves the shape. `None` for other transforms.
  pub shifts: Vec<Option<ShiftLine>>,
  /// Where the shape's center is going into each transform, in the space the ones before it put the shape in, then in
  /// scene space after the last. A `Rotate` turns around it.
  pub pivots: Vec<Vec2>,
  /// For each transform, the center of a `Mirror`, `Revolve`, or `Divide`, in the space the ones before it put the shape
  /// in. `None` for other transforms.
  pub centers: Vec<Option<Vec2>>,
  /// Outline of the shape itself in scene space, without what a `Mirror`, `Revolve`, or `Divide` does to it. Closed,
  /// except for polylines and beziers.
  pub outline: Vec<Vec2>,
  /// For each transform, the pose of the `Sweep` path it follows, if any. The path's coordinates are translate values.
  pub paths: Vec<Option<ShapePose>>,
}

impl Shape {
  /// The shape as it is in `env`, to cover any number of samples with. `None` if it's collapsed.
  pub(crate) fn prepare(&self, env: &Env) -> Option<PreparedShape> {
    let bounds = self.base.bounds(env);
    let to_scene = self.to_scene(bounds.center(), env)?;
    let edge = match self.shifted(env) {
      Some((outline, _)) if !self.base.is_open() => Edge::Polygon(outline),
      _ => self.base.edge(env, to_scene),
    };
    let wrap = self.wrap(bounds.center(), to_scene, env);
    let extent = edge.extent();
    let strokes: Vec<PreparedStroke> = self.adjust.iter().map(|adjust| adjust.prepare(env)).collect();
    // each band can reach past the edge of the one before
    let stroke_reach = strokes.iter().map(PreparedStroke::reach).sum();
    let copies = self
      .placements(bounds.center(), env)
      .into_iter()
      .map(|placed| PreparedCopy {
        to_local: placed.to_scene.inverse(),
        reach: wrap.is_none().then(|| extent.transformed(placed.to_scene).grown(stroke_reach)),
        cuts: placed.cuts,
      })
      .collect();
    let colored = strokes.iter().filter(|stroke| stroke.color.is_some()).count();
    Some(PreparedShape { edge, bounds, to_local: to_scene.inverse(), wrap, copies, strokes, colored })
  }

  /// Outline in scene space, as a closed loop for each copy of the shape, with what's cut from it left off. A polyline
  /// or bezier isn't cut, and its loops don't close. Empty if the shape is collapsed.
  pub(crate) fn outline(&self, env: &Env) -> Vec<Vec<Vec2>> {
    let bounds = self.base.bounds(env);
    if self.to_scene(bounds.center(), env).is_none() {
      return Vec::new();
    }
    let own = self.own_outline(env);
    let open = self.base.is_open();
    let placed = self.placements(bounds.center(), env).into_iter().map(|placed| {
      let outline: Vec<Vec2> = own.iter().map(|p| placed.to_scene.transform_point2(*p)).collect();
      if open { outline } else { placed.cuts.iter().fold(outline, |outline, cut| clipped(&outline, cut)) }
    });
    placed.filter(|outline| !outline.is_empty()).collect()
  }

  /// `None` if a transform collapses the shape
  pub(crate) fn pose(&self, env: &Env) -> Option<ShapePose> {
    let bounds = self.base.bounds(env);
    let to_scene = self.to_scene(bounds.center(), env)?;
    let steps = self.steps(bounds.center(), env);
    let last = self.transform.last().zip(steps.last());
    let end = last.map_or(bounds.center(), |(transform, (affine, center))| transform.center(*center, *affine, env));
    let points = match &self.base {
      ShapeNode::Polygon { points } | ShapeNode::Polyline { points } => points.iter().map(|p| p.resolve(env)).collect(),
      ShapeNode::Bezier { points } => points.iter().map(|p| p.point.resolve(env)).collect(),
      _ => Vec::new(),
    };
    let handles = match &self.base {
      ShapeNode::Bezier { points } => points.iter().map(|p| p.handle.resolve(env)).collect(),
      _ => Vec::new(),
    };
    let (outline, shifts) = self.shifted(env).unwrap_or_else(|| (self.base.outline(env), vec![None; self.transform.len()]));
    Some(ShapePose {
      transforms: steps.iter().map(|(affine, _)| *affine).collect(),
      bounds,
      points,
      handles,
      pivots: steps.iter().map(|(_, center)| *center).chain([end]).collect(),
      centers: self.transform.iter().map(|transform| transform.copy_center(env)).collect(),
      shifts,
      outline: outline.into_iter().map(|p| to_scene.transform_point2(p)).collect(),
      paths: self.transform.iter().map(|transform| transform.path().and_then(|path| path.pose(env))).collect(),
    })
  }

  /// The base's outline in local space, with each `Shift` applied
  fn own_outline(&self, env: &Env) -> Vec<Vec2> {
    self.shifted(env).map_or_else(|| self.base.outline(env), |(outline, _)| outline)
  }

  /// The base's outline in local space with each `Shift` applied, and for each transform, where a `Shift` moves the
  /// shape. `None` without a shift, or for text, which isn't shifted.
  fn shifted(&self, env: &Env) -> Option<(Vec<Vec2>, Vec<Option<ShiftLine>>)> {
    if matches!(self.base, ShapeNode::Text { .. }) || self.transform.iter().all(|transform| transform.shift(env).is_none()) {
      return None;
    }
    let mut outline = self.base.outline(env);
    // local space to the one each transform starts from
    let mut before = Affine2::IDENTITY;
    let mut lines = Vec::with_capacity(self.transform.len());
    for (transform, (affine, center)) in self.transform.iter().zip(self.steps(self.base.bounds(env).center(), env)) {
      let shift = transform.shift(env);
      if let Some(shift) = shift.filter(|_| before.matrix2.determinant().abs() > f32::EPSILON) {
        let placed: Vec<Vec2> = outline.iter().map(|p| before.transform_point2(*p)).collect();
        lines.push(Some(shift.line(&placed, center)));
        outline = shift.apply(&outline, !self.base.is_open(), before, center);
      } else {
        lines.push(None);
      }
      before = affine * before;
    }
    Some((outline, lines))
  }

  /// Each copy of the shape its `Mirror`s and `Revolve`s draw, the shape itself first, with what its `Divide`s cut from
  /// it. Assumes the shape isn't collapsed.
  fn placements(&self, center: Vec2, env: &Env) -> Vec<Placement> {
    let mut placed = vec![Placement { to_scene: Affine2::IDENTITY, cuts: Vec::new() }];
    for (transform, affine) in self.transform.iter().zip(self.affines(center, env)) {
      let moves = transform.copies(env).unwrap_or_else(|| vec![affine]);
      placed = placed.iter().flat_map(|placement| moves.iter().map(|moved| placement.moved(*moved))).take(MAX_COPIES).collect();
      if let Some(cut) = transform.cut(env) {
        placed.iter_mut().for_each(|placement| placement.cuts.push(cut));
      }
    }
    placed
  }

  /// The scene-space point `phase` (0 to 1) of the way along the outline. The origin if there's no outline.
  pub(crate) fn along(&self, phase: f32, env: &Env) -> Vec2 {
    let own = match self.shifted(env) {
      Some((outline, _)) if self.base.is_open() => along_line(&outline, phase),
      Some((outline, _)) => along_loop(&outline, phase),
      None => self.base.along(phase, env),
    };
    match self.to_scene(self.base.bounds(env).center(), env) {
      Some(to_scene) => to_scene.transform_point2(own),
      None => Vec2::ZERO,
    }
  }

  pub(crate) fn is_animated(&self) -> bool {
    self.base.is_animated() || self.transform.iter().any(Transform::is_animated) || self.adjust.iter().any(ShapeAdjust::is_animated)
  }
}

impl ShapeNode {
  pub fn circle(radius: f32) -> Self {
    Self::Circle { radius: radius.into() }
  }

  /// Text centered in a box `frame` wide and tall
  pub fn text(text: impl Into<TextSource>, font: impl Into<String>, size: f32, frame: Vec2) -> Self {
    Self::Text {
      text: text.into(),
      font: font.into(),
      size: size.into(),
      width: frame.x.into(),
      height: frame.y.into(),
      align: TextAlign::default(),
      vertical_align: VerticalAlign::default(),
    }
  }

  /// The name of the font it draws text in, if it's text
  pub fn font_name(&self) -> Option<&str> {
    match self {
      Self::Text { font, .. } => Some(font),
      _ => None,
    }
  }

  fn is_animated(&self) -> bool {
    match self {
      Self::Circle { radius } => radius.is_animated(),
      Self::Ellipse { rx: a, ry: b } | Self::Rect { width: a, height: b } => a.is_animated() || b.is_animated(),
      Self::Chevron { width: a, height: b, notch: c } | Self::Diamond { width: a, height: b, bow: c } => {
        a.is_animated() || b.is_animated() || c.is_animated()
      }
      Self::Star { radius: a, legs: b, leg_width: c, bow: d } => {
        a.is_animated() || b.is_animated() || c.is_animated() || d.is_animated()
      }
      Self::Polygon { points } | Self::Polyline { points } => points.iter().any(Prop::is_animated),
      Self::Bezier { points } => points.iter().any(|p| p.point.is_animated() || p.handle.is_animated()),
      Self::Text { text, size, width, height, .. } => text.is_animated() || size.is_animated() || width.is_animated() || height.is_animated(),
    }
  }

  /// Bounds in local space, before transforms
  pub(crate) fn bounds(&self, env: &Env) -> Bounds {
    match self {
      Self::Circle { .. }
      | Self::Ellipse { .. }
      | Self::Rect { .. }
      | Self::Diamond { .. }
      | Self::Star { .. }
      | Self::Chevron { .. }
      | Self::Text { .. } => {
        let reach = self.reach(env);
        Bounds::new(-reach, reach)
      }
      Self::Polygon { points } | Self::Polyline { points } => bounds_of(points.iter().map(|p| p.resolve(env))),
      // points only, so dragging a handle doesn't move the center that `Rotate` pivots on
      Self::Bezier { points } => bounds_of(points.iter().map(|p| p.point.resolve(env))),
    }
  }

  /// Outline in local space, before transforms
  fn outline(&self, env: &Env) -> Vec<Vec2> {
    match self {
      Self::Circle { .. } | Self::Ellipse { .. } => (0..ROUND_OUTLINE_POINTS)
        .map(|i| Vec2::from_angle(std::f32::consts::TAU * i as f32 / ROUND_OUTLINE_POINTS as f32) * self.reach(env))
        .collect(),
      // text's is its box, as tracing every letter is more than an outline shows
      Self::Rect { .. } | Self::Text { .. } => {
        let reach = self.reach(env);
        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].map(|(x, y)| Vec2::new(x, y) * reach).to_vec()
      }
      Self::Diamond { bow, .. } => {
        let Vec2 { x, y } = self.reach(env);
        let bow = bow.resolve(env);
        let corners = [Vec2::new(0.0, -y), Vec2::new(x, 0.0), Vec2::new(0.0, y), Vec2::new(-x, 0.0)];
        (0..4).flat_map(|i| bowed(corners[i], corners[(i + 1) % 4], bow)).collect()
      }
      Self::Star { radius, legs, leg_width, bow } => {
        let (radius, legs, bow) = (radius.resolve(env), whole_legs(legs, env), bow.resolve(env));
        let corners = star_outline(legs, radius, radius * leg_width.resolve(env) / 2.0);
        // one leg's last side is its base, across the origin
        let sides = if legs == 1 { 2 } else { corners.len() };
        let side = |i: usize| if i < sides { bowed(corners[i], corners[(i + 1) % corners.len()], bow) } else { vec![corners[i]] };
        (0..corners.len()).flat_map(side).collect()
      }
      Self::Chevron { notch, .. } => {
        let Vec2 { x, y } = self.reach(env);
        let notch = y - 2.0 * y * unit_fraction(notch, env);
        vec![Vec2::new(0.0, -y), Vec2::new(x, y), Vec2::new(0.0, notch), Vec2::new(-x, y)]
      }
      // too few points to enclose anything, so it draws nothing
      Self::Polygon { points } if points.len() < 3 => Vec::new(),
      Self::Polygon { points } | Self::Polyline { points } => points.iter().map(|p| p.resolve(env)).collect(),
      Self::Bezier { points } => bezier_line(points, env),
    }
  }

  /// The local-space point `phase` of the way along the outline. Round shapes use the angle directly instead of building
  /// the outline polygon, since a sweep runs this for every sample.
  fn along(&self, phase: f32, env: &Env) -> Vec2 {
    match self {
      Self::Circle { .. } | Self::Ellipse { .. } => {
        Vec2::from_angle(std::f32::consts::TAU * phase) * self.reach(env)
      }
      Self::Polyline { .. } | Self::Bezier { .. } => along_line(&self.outline(env), phase),
      _ => along_loop(&self.outline(env), phase),
    }
  }

  /// Whether it's a line, with no inside
  fn is_open(&self) -> bool {
    matches!(self, Self::Polyline { .. } | Self::Bezier { .. })
  }

  /// Half-extents of a shape that isn't made of points or text. Zero for those that are.
  fn reach(&self, env: &Env) -> Vec2 {
    match self {
      Self::Circle { radius } => Vec2::splat(radius.resolve(env)),
      Self::Ellipse { rx, ry } => Vec2::new(rx.resolve(env), ry.resolve(env)),
      Self::Rect { width, height }
      | Self::Diamond { width, height, .. }
      | Self::Chevron { width, height, .. }
      | Self::Text { width, height, .. } => Vec2::new(width.resolve(env), height.resolve(env)) / 2.0,
      Self::Star { radius, .. } => Vec2::splat(radius.resolve(env).max(0.0)),
      Self::Polygon { .. } | Self::Polyline { .. } | Self::Bezier { .. } => Vec2::ZERO,
    }
  }

  /// The edge with its sizes resolved. `to_scene` is where the shape is drawn, which text's lines are lined up with.
  fn edge(&self, env: &Env, to_scene: Affine2) -> Edge {
    match self {
      Self::Circle { .. } | Self::Ellipse { .. } => Edge::Ellipse(self.reach(env)),
      Self::Rect { .. } => Edge::Rect(self.reach(env)),
      Self::Diamond { .. } | Self::Star { .. } | Self::Chevron { .. } => Edge::Polygon(self.outline(env)),
      Self::Polygon { points } => Edge::Polygon(points.iter().map(|p| p.resolve(env)).collect()),
      Self::Text { text, font, size, align, vertical_align, .. } => match env.assets.font(font) {
        Some(font) => {
          let reach = self.reach(env);
          let moved_to = to_scene.matrix2.abs_diff_eq(Mat2::IDENTITY, 1e-6).then_some(to_scene.translation);
          Edge::Text(layout_text(font, &text.resolve(env), size.resolve(env), reach * 2.0, *align, *vertical_align, moved_to), reach)
        }
        None => Edge::Open,
      },
      Self::Polyline { .. } | Self::Bezier { .. } => Edge::Open,
    }
  }
}

/// A [`Shape`] resolved for one frame, with what covering a sample needs
pub(crate) struct PreparedShape {
  edge: Edge,
  /// Bounds in local space, before transforms
  pub bounds: Bounds,
  /// Scene to local space for the shape itself, without the copies a `Mirror` or `Revolve` makes
  pub to_local: Affine2,
  wrap: Option<Wrap>,
  /// Each copy that's drawn, the shape itself first
  copies: Vec<PreparedCopy>,
  /// In order
  strokes: Vec<PreparedStroke>,
  /// How many of the strokes have a color
  colored: usize,
}

impl PreparedShape {
  /// How much of the sample the shape and its colored strokes cover. `None` if the sample is outside every copy.
  pub(crate) fn cover(&self, at: &Sample) -> Option<Cover> {
    let pos = self.wrap.map_or(at.pos, |wrap| wrap.apply(at.pos));
    let mut nearest: Option<(f32, Vec2)> = None;
    // the nearest copy's distance to each colored stroke's band
    let mut bands = vec![f32::INFINITY; self.colored];
    for copy in &self.copies {
      if copy.reach.is_some_and(|reach| !reach.grown(at.radius).contains(at.pos)) {
        continue;
      }
      let local = copy.to_local.transform_point2(pos);
      let (distance, gradient) = self.edge.distance(local);
      // inside the edge and on the kept side of every cut, so the farthest of them
      let mut distance = copy.cuts.iter().fold(scene_distance(distance, gradient, &copy.to_local), |distance, cut| distance.max(cut.distance(pos)));
      let mut colored = bands.iter_mut();
      for stroke in &self.strokes {
        if stroke.color.is_none() {
          distance = stroke.band(distance);
        } else if let Some(band) = colored.next() {
          *band = band.min(stroke.band(distance));
        }
      }
      if nearest.is_none_or(|(nearest, _)| distance < nearest) {
        nearest = Some((distance, local));
      }
    }
    let (distance, local) = nearest?;
    let colors = self.strokes.iter().filter_map(|stroke| stroke.color);
    let strokes: Vec<(f32, Rgba<u8>)> =
      bands.into_iter().zip(colors).map(|(band, color)| (coverage(band, at.radius), color)).filter(|(amount, _)| *amount > 0.0).collect();
    let amount = coverage(distance, at.radius);
    (amount > 0.0 || !strokes.is_empty()).then_some(Cover { amount, local, strokes })
  }
}

/// How much of a sample a [`PreparedShape`] covers
pub(crate) struct Cover {
  /// How much the shape covers, 0 to 1
  pub amount: f32,
  /// The sample's position in the local space of the copy nearest it
  pub local: Vec2,
  /// Each colored stroke that covers some of the sample, in order: how much, and its color
  pub strokes: Vec<(f32, Rgba<u8>)>,
}

/// One copy of a [`PreparedShape`]
struct PreparedCopy {
  to_local: Affine2,
  /// In scene space
  cuts: Vec<Cut>,
  /// Everything it can cover, in scene space. `None` when it wraps around a `Sequence`.
  reach: Option<Bounds>,
}

/// Where one copy of a shape is drawn, from local to scene space, and what's cut from it in scene space
struct Placement {
  to_scene: Affine2,
  cuts: Vec<Cut>,
}

impl Placement {
  /// The same copy, and its cuts, moved on by `affine`
  fn moved(&self, affine: Affine2) -> Self {
    Self { to_scene: affine * self.to_scene, cuts: self.cuts.iter().map(|cut| cut.transformed(affine)).collect() }
  }
}

/// A shape's edge in local space, with its sizes resolved
enum Edge {
  Ellipse(Vec2),
  Rect(Vec2),
  Polygon(Vec<Vec2>),
  /// Laid out in a box, with half its width and height, that it's cut off at
  Text(TextOutline, Vec2),
  /// A polyline or bezier, which has no inside
  Open,
}

impl Edge {
  /// Signed distance to the edge (negative inside), and its gradient
  fn distance(&self, p: Vec2) -> (f32, Vec2) {
    match self {
      Self::Ellipse(reach) => ellipse_distance(*reach, p),
      Self::Rect(reach) => rect_distance(*reach, p),
      Self::Polygon(points) => polygon_distance(points, p),
      // inside the letters and inside the box, so the farther of the two
      Self::Text(outline, reach) => {
        let (text, frame) = (outline.distance(p), rect_distance(*reach, p));
        if frame.0 > text.0 { frame } else { text }
      }
      Self::Open => (f32::INFINITY, Vec2::X),
    }
  }

  /// A box around everything inside the edge. Wider than the shape's bounds for a star or diamond with its sides
  /// bowed out.
  fn extent(&self) -> Bounds {
    let around = |reach: Vec2| Bounds::new(-reach, reach);
    match self {
      Self::Ellipse(reach) | Self::Rect(reach) => around(reach.abs()),
      Self::Polygon(points) => bounds_of(points.iter().copied()),
      Self::Text(outline, reach) => {
        let extent = outline.extent();
        Bounds::new(extent.min.max(-*reach), extent.max.min(*reach))
      }
      Self::Open => Bounds::new(Vec2::ZERO, Vec2::ZERO),
    }
  }
}

fn bounds_of(mut points: impl Iterator<Item = Vec2>) -> Bounds {
  let Some(first) = points.next() else {
    return Bounds::new(Vec2::ZERO, Vec2::ZERO);
  };
  points.fold(Bounds::new(first, first), |rect, p| Bounds::new(rect.min.min(p), rect.max.max(p)))
}

/// The point `phase` of the way around a closed polygon, by distance
fn along_loop(points: &[Vec2], phase: f32) -> Vec2 {
  let closed: Vec<Vec2> = points.iter().chain(points.first()).copied().collect();
  along_line(&closed, phase)
}

/// The point `phase` of the way along an open polyline, by distance
fn along_line(points: &[Vec2], phase: f32) -> Vec2 {
  let edges = || points.windows(2).map(|edge| (edge[0], edge[1]));
  let mut left = phase * edges().map(|(a, b)| a.distance(b)).sum::<f32>();
  for (a, b) in edges() {
    let length = a.distance(b);
    if left <= length && length > 0.0 {
      return a.lerp(b, left / length);
    }
    left -= length;
  }
  points.last().copied().unwrap_or_default()
}

/// Flattens a bezier into a polyline
fn bezier_line(points: &[BezierPoint], env: &Env) -> Vec<Vec2> {
  let resolved: Vec<(Vec2, Vec2)> = points.iter().map(|p| (p.point.resolve(env), p.handle.resolve(env))).collect();
  let stretches = resolved.windows(2).flat_map(|stretch| {
    let ((from, leaving), (to, arriving)) = (stretch[0], stretch[1]);
    (1..=BEZIER_STEPS).map(move |step| cubic([from, from + leaving, to - arriving, to], step as f32 / BEZIER_STEPS as f32))
  });
  resolved.first().map(|(first, _)| *first).into_iter().chain(stretches).collect()
}

/// The point at `t` on a cubic bezier (de Casteljau)
pub(crate) fn cubic([a, b, c, d]: [Vec2; 4], t: f32) -> Vec2 {
  let (e, f, g) = (a.lerp(b, t), b.lerp(c, t), c.lerp(d, t));
  e.lerp(f, t).lerp(f.lerp(g, t), t)
}

/// Converts a local-space signed distance to scene units, using its gradient. Exact under uniform scale; under
/// non-uniform scale it's only accurate near the edge, which is all coverage needs.
fn scene_distance(distance: f32, gradient: Vec2, to_local: &Affine2) -> f32 {
  let stretch = (to_local.matrix2.transpose() * gradient).length();
  if stretch > f32::EPSILON { distance / stretch } else { distance }
}

/// Anti-aliased coverage: a linear ramp across the sample's radius, so a sample centered on the edge is half covered
fn coverage(distance: f32, radius: f32) -> f32 {
  if radius <= 0.0 {
    return if distance <= 0.0 { 1.0 } else { 0.0 };
  }
  (0.5 - distance / (2.0 * radius)).clamp(0.0, 1.0)
}

/// Approximate signed distance to an ellipse's edge, as a fraction of its radius, with its gradient. Accurate near the
/// edge once divided by the gradient's length (see `scene_distance`).
fn ellipse_distance(reach: Vec2, p: Vec2) -> (f32, Vec2) {
  if reach.min_element() <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let along = p / reach;
  (along.length() - 1.0, along.normalize_or(Vec2::X) / reach)
}

fn whole_legs(legs: &Prop<f32>, env: &Env) -> u32 {
  legs.resolve_truncated(env).floor().clamp(0.0, MAX_LEGS) as u32
}

fn unit_fraction(fraction: &Prop<f32>, env: &Env) -> f32 {
  fraction.resolve(env).clamp(0.0, 1.0)
}

/// The points along a side from `from` toward `to`, leaving out `to` so that sides can be joined end to end. With a
/// `bow`, the side curves away from the origin (or toward it, when negative) by that fraction of its length at its
/// middle.
fn bowed(from: Vec2, to: Vec2, bow: f32) -> Vec<Vec2> {
  if bow == 0.0 {
    return vec![from];
  }
  let middle = (from + to) / 2.0;
  let across = (to - from).perp();
  let out = if across.dot(middle) < 0.0 { -across } else { across };
  // a quadratic bezier reaches halfway to its control point at its middle
  let control = middle + out * bow * 2.0;
  (0..BOW_STEPS)
    .map(|step| {
      let t = step as f32 / BOW_STEPS as f32;
      from.lerp(control, t).lerp(control.lerp(to, t), t)
    })
    .collect()
}

/// Each leg's tip, then where its side meets the next leg's, going clockwise from the leg that points up. `side` is
/// half a leg's width. One leg has no neighbor, so it's a triangle with its base across the origin.
fn star_outline(legs: u32, radius: f32, side: f32) -> Vec<Vec2> {
  if legs == 0 || radius <= 0.0 {
    return Vec::new();
  }
  let up = -std::f32::consts::FRAC_PI_2;
  let share = std::f32::consts::TAU / legs as f32;
  if legs == 1 {
    return vec![Vec2::new(-side, 0.0), Vec2::new(0.0, -radius), Vec2::new(side, 0.0)];
  }
  // a leg's side runs from `side` across the origin to its tip, so it crosses the line halfway to the next leg where
  // `cos / radius + sin / side` reaches 1 over the distance out
  let half = share / 2.0;
  let between = if side > 0.0 { 1.0 / (half.cos() / radius + half.sin() / side) } else { 0.0 };
  (0..legs)
    .flat_map(|leg| {
      let angle = up + leg as f32 * share;
      [Vec2::from_angle(angle) * radius, Vec2::from_angle(angle + half) * between]
    })
    .collect()
}

/// A closed outline with the part on the far side of `cut` taken off (Sutherland–Hodgman)
fn clipped(outline: &[Vec2], cut: &Cut) -> Vec<Vec2> {
  let mut kept = Vec::with_capacity(outline.len() + 1);
  for (i, &to) in outline.iter().enumerate() {
    let from = outline[(i + outline.len() - 1) % outline.len()];
    let (from_distance, to_distance) = (cut.distance(from), cut.distance(to));
    if (from_distance <= 0.0) != (to_distance <= 0.0) {
      kept.push(from.lerp(to, from_distance / (from_distance - to_distance)));
    }
    if to_distance <= 0.0 {
      kept.push(to);
    }
  }
  kept
}

fn rect_distance(reach: Vec2, p: Vec2) -> (f32, Vec2) {
  if reach.min_element() <= 0.0 {
    return (f32::INFINITY, Vec2::X);
  }
  let past = p.abs() - reach;
  let side = Vec2::new(p.x.signum(), p.y.signum());
  if past.max_element() > 0.0 {
    let outside = past.max(Vec2::ZERO);
    return (outside.length(), (outside * side).normalize_or(Vec2::X));
  }
  if past.x > past.y { (past.x, Vec2::X * side.x) } else { (past.y, Vec2::Y * side.y) }
}

fn polygon_distance(points: &[Vec2], p: Vec2) -> (f32, Vec2) {
  if points.len() < 3 {
    return (f32::INFINITY, Vec2::X);
  }

  let mut nearest = p - points[0];
  let mut nearest_sq = nearest.length_squared();
  let mut sign = 1.0;

  let mut j = points.len() - 1;
  for i in 0..points.len() {
    let (a, b) = (points[i], points[j]);
    let edge = b - a;
    let to_p = p - a;

    let length_sq = edge.length_squared();
    let along = if length_sq > 0.0 { (to_p.dot(edge) / length_sq).clamp(0.0, 1.0) } else { 0.0 };
    let offset = to_p - edge * along;
    if offset.length_squared() < nearest_sq {
      nearest = offset;
      nearest_sq = offset.length_squared();
    }

    // flip the sign each time a ray from p crosses an edge
    let crosses = [p.y >= a.y, p.y < b.y, edge.x * to_p.y > edge.y * to_p.x];
    if crosses.iter().all(|c| *c) || crosses.iter().all(|c| !*c) {
      sign = -sign;
    }
    j = i;
  }

  (sign * nearest_sq.sqrt(), sign * nearest.normalize_or(Vec2::X))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::scene::{Color, Key, Keyframes, Params, StrokeAlign, Transform};
  use crate::scene::value::tests::env;

  fn square() -> ShapeNode {
    rect_polygon(1.0, 1.0)
  }

  fn rect_polygon(width: f32, height: f32) -> ShapeNode {
    ShapeNode::Polygon {
      points: [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)]
        .into_iter()
        .map(|(x, y)| Vec2::new(x * width, y * height).into())
        .collect(),
    }
  }

  fn covered(shape: &Shape, x: f32, y: f32, radius: f32) -> f32 {
    let params = Params::new();
    let at = Sample::new(Vec2::new(x, y), radius);
    shape.prepare(&env(0.0, &params)).and_then(|shape| shape.cover(&at)).map_or(0.0, |cover| cover.amount)
  }

  #[test]
  fn circle_covers_inside_and_not_outside() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(10.0.into()));
    assert_eq!(covered(&circle, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 4.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&circle, 6.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn edge_through_a_sample_covers_it_partially() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(10.0.into()));
    assert!((covered(&circle, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&circle, 5.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn an_ellipse_edge_is_in_scene_units() {
    let ellipse = Shape::new(ShapeNode::Ellipse { rx: 10.0.into(), ry: 2.0.into() });
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 10.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn a_polygon_covers_its_inside() {
    let rect = Shape::new(rect_polygon(10.0, 4.0));
    assert_eq!(covered(&rect, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&rect, 4.0, 1.5, 0.5), 1.0);
    assert_eq!(covered(&rect, 0.0, 3.0, 0.5), 0.0);
    assert!((covered(&rect, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn polygon_winding_does_not_matter() {
    let ShapeNode::Polygon { mut points } = square() else { unreachable!() };
    points.reverse();
    let shape = Shape::new(ShapeNode::Polygon { points });
    assert_eq!(covered(&shape, 0.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 1.0, 0.0, 0.1), 0.0);
  }

  #[test]
  fn polygon_bounds() {
    let params = Params::new();
    let triangle = ShapeNode::Polygon {
      points: vec![Vec2::new(0.0, 0.0).into(), Vec2::new(4.0, 1.0).into(), Vec2::new(1.0, 3.0).into()],
    };
    assert_eq!(triangle.bounds(&env(0.0, &params)), Bounds::new(Vec2::ZERO, Vec2::new(4.0, 3.0)));
  }

  #[test]
  fn an_ellipses_radii_are_up_and_down_then_side_to_side() {
    let ellipse = Shape::new(ShapeNode::Ellipse { rx: 10.0.into(), ry: 2.0.into() });
    assert_eq!(covered(&ellipse, 9.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&ellipse, 0.0, 3.0, 0.1), 0.0);
    assert!((covered(&ellipse, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&ellipse, 0.0, 2.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn an_ellipse_with_no_radius_draws_nothing() {
    let ellipse = Shape::new(ShapeNode::Ellipse { rx: 10.0.into(), ry: 0.0.into() });
    assert_eq!(covered(&ellipse, 0.0, 0.0, 0.5), 0.0);
  }

  fn stroked(shape: ShapeNode, width: f32, align: StrokeAlign) -> Shape {
    Shape::new(shape).adjust(ShapeAdjust::Stroke { width: width.into(), align, color: None })
  }

  fn ring(radius: f32, thickness: f32) -> Shape {
    stroked(ShapeNode::circle(radius), thickness, StrokeAlign::Outside)
  }

  #[test]
  fn a_circle_reaches_its_radius_every_way() {
    let params = Params::new();
    let circle = ShapeNode::circle(3.0);
    assert_eq!(circle.bounds(&env(0.0, &params)), Bounds::new(Vec2::splat(-3.0), Vec2::splat(3.0)));
  }

  #[test]
  fn a_stroke_with_no_color_covers_its_band_and_not_the_inside() {
    let band = ring(4.0, 2.0);
    assert_eq!(covered(&band, 0.0, 0.0, 0.1), 0.0);
    assert_eq!(covered(&band, 3.0, 0.0, 0.1), 0.0);
    assert_eq!(covered(&band, 0.0, 5.0, 0.1), 1.0);
    assert_eq!(covered(&band, 7.0, 0.0, 0.1), 0.0);
    assert!((covered(&band, 4.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&band, 6.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn a_stroke_is_aligned_inside_across_or_outside_the_edge() {
    let rect = || ShapeNode::Rect { width: 20.0.into(), height: 20.0.into() };
    for (align, from, to) in [(StrokeAlign::Inside, 8.0, 10.0), (StrokeAlign::Centered, 9.0, 11.0), (StrokeAlign::Outside, 10.0, 12.0)] {
      let band = stroked(rect(), 2.0, align);
      assert_eq!(covered(&band, (from + to) / 2.0, 0.0, 0.1), 1.0, "{align:?}");
      assert_eq!(covered(&band, from - 0.5, 0.0, 0.1), 0.0, "{align:?}");
      assert_eq!(covered(&band, to + 0.5, 0.0, 0.1), 0.0, "{align:?}");
    }
  }

  #[test]
  fn a_stroke_is_in_scene_units_however_the_shape_is_scaled() {
    let band = stroked(ShapeNode::circle(0.4), 2.0, StrokeAlign::Outside).transform(Transform::Scale(10.0.into()));
    assert!((covered(&band, 4.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&band, 6.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn a_stroke_with_no_width_draws_nothing() {
    assert_eq!(covered(&ring(4.0, 0.0), 4.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn a_stroke_outlines_each_copy() {
    // two circles that overlap, so the edge of one is inside the other
    let shape = dot((1.5, 0.0), mirror((0.0, 0.0), 0.0)).adjust(ShapeAdjust::Stroke { width: 0.5.into(), align: StrokeAlign::Centered, color: None });
    assert_eq!(covered(&shape, -0.5, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.5, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, 0.0, 0.1), 0.0);
  }

  #[test]
  fn a_stroke_with_a_color_is_drawn_apart_from_the_shape() {
    let params = Params::new();
    let red = Rgba([255, 0, 0, 255]);
    let shape = Shape::new(ShapeNode::circle(4.0))
      .adjust(ShapeAdjust::Stroke { width: 2.0.into(), align: StrokeAlign::Outside, color: Some(Color(red).into()) })
      .prepare(&env(0.0, &params))
      .unwrap();
    let inside = shape.cover(&Sample::new(Vec2::ZERO, 0.1)).unwrap();
    assert_eq!((inside.amount, inside.strokes), (1.0, vec![]));
    let band = shape.cover(&Sample::new(Vec2::new(5.0, 0.0), 0.1)).unwrap();
    assert_eq!((band.amount, band.strokes), (0.0, vec![(1.0, red)]));
    assert!(shape.cover(&Sample::new(Vec2::new(7.0, 0.0), 0.1)).is_none());
  }

  #[test]
  fn a_colored_stroke_after_one_with_no_color_outlines_its_band() {
    let params = Params::new();
    let red = Rgba([255, 0, 0, 255]);
    let shape = ring(4.0, 2.0)
      .adjust(ShapeAdjust::Stroke { width: 0.5.into(), align: StrokeAlign::Outside, color: Some(Color(red).into()) })
      .prepare(&env(0.0, &params))
      .unwrap();
    for x in [3.75, 6.25] {
      let edge = shape.cover(&Sample::new(Vec2::new(x, 0.0), 0.1)).unwrap();
      assert_eq!((edge.amount, edge.strokes), (0.0, vec![(1.0, red)]), "at {x}");
    }
  }

  #[test]
  fn a_stroke_can_be_animated() {
    let grows = Prop::Keyframes(Keyframes { keys: vec![Key::new(0.0, 1.0, Curve::Linear)], cycle: Cycle::Once });
    assert!(Shape::new(ShapeNode::circle(1.0)).adjust(ShapeAdjust::Stroke { width: grows, align: StrokeAlign::Centered, color: None }).is_animated());
    assert!(!ring(4.0, 2.0).is_animated());
  }

  fn diamond() -> Shape {
    Shape::new(ShapeNode::Diamond { width: 20.0.into(), height: 20.0.into(), bow: 0.0.into() })
  }

  fn shift(amount: f32, rotation: f32) -> Transform {
    Transform::Shift { amount: amount.into(), rotation: rotation.into() }
  }

  #[test]
  fn a_diamond_is_widest_halfway_down() {
    let shape = diamond();
    for (x, y) in [(0.0, 0.0), (9.0, 0.0), (-9.0, 0.0), (0.0, 9.0), (0.0, -9.0)] {
      assert_eq!(covered(&shape, x, y, 0.1), 1.0, "at ({x}, {y})");
    }
    assert_eq!(covered(&shape, 6.0, 6.0, 0.1), 0.0);
    assert_eq!(covered(&shape, -6.0, -6.0, 0.1), 0.0);
  }

  #[test]
  fn a_shift_moves_a_diamonds_widest_point_up_and_down() {
    let shape = diamond().transform(shift(-0.5, 0.0));
    assert_eq!(covered(&shape, 9.0, -5.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 9.0, 0.0, 0.1), 0.0);
    assert_eq!(covered(&shape, 0.0, 9.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, -9.0, 0.1), 1.0);
  }

  #[test]
  fn a_shift_all_the_way_squashes_one_half_flat() {
    let shape = diamond().transform(shift(1.0, 0.0));
    // a triangle pointing up, its base along the bottom
    assert_eq!(covered(&shape, 9.0, 9.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, -9.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 6.0, -6.0, 0.1), 0.0);
  }

  #[test]
  fn a_shift_turns_with_its_rotation() {
    // at 90 degrees the line points left, so the corners at the top and bottom move that way
    let shape = diamond().transform(shift(0.5, 90.0));
    assert_eq!(covered(&shape, -5.0, 9.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, 9.0, 0.1), 0.0);
  }

  #[test]
  fn a_shift_bends_a_straight_side_where_it_crosses_the_middle() {
    // a square turned on its side is a diamond, and shifts as one
    let square = Shape::new(ShapeNode::Rect { width: 20.0.into(), height: 20.0.into() });
    let shape = square.transform(Transform::Rotate(45.0.into())).transform(shift(-0.5, 0.0));
    let reach = 10.0 * std::f32::consts::SQRT_2;
    assert_eq!(covered(&shape, reach - 1.0, -reach / 2.0, 0.1), 1.0);
    assert_eq!(covered(&shape, reach - 1.0, 0.0, 0.1), 0.0);
  }

  #[test]
  fn a_shift_moves_in_the_space_the_transforms_before_it_leave() {
    let shape = diamond().transform(Transform::Translate(Vec2::new(100.0, 50.0).into())).transform(shift(-0.5, 0.0));
    assert_eq!(covered(&shape, 109.0, 45.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 109.0, 50.0, 0.1), 0.0);
  }

  #[test]
  fn text_isnt_shifted_and_its_outline_is_its_box() {
    let params = Params::new();
    let text = Shape::new(ShapeNode::text("A", "nothing", 8.0, Vec2::new(20.0, 10.0))).transform(shift(1.0, 0.0));
    let outline = text.pose(&env(0.0, &params)).unwrap().outline;
    assert_eq!(outline.len(), 4);
  }

  #[test]
  fn a_diamonds_sides_bow() {
    let bowed = Shape::new(ShapeNode::Diamond { width: 20.0.into(), height: 20.0.into(), bow: 0.25.into() });
    assert_eq!(covered(&diamond(), 6.0, 6.0, 0.1), 0.0);
    assert_eq!(covered(&bowed, 6.0, 6.0, 0.1), 1.0);
  }

  fn star(legs: f32) -> Shape {
    bowed_star(legs, 0.0)
  }

  fn bowed_star(legs: f32, bow: f32) -> Shape {
    Shape::new(ShapeNode::Star { radius: 20.0.into(), legs: legs.into(), leg_width: 0.4.into(), bow: bow.into() })
  }

  #[test]
  fn a_stars_first_leg_points_up_and_the_rest_go_around() {
    let shape = star(5.0);
    let at = |degrees: f32, length: f32| Vec2::from_angle((degrees - 90.0).to_radians()) * length;
    for leg in 0..5 {
      let tip = at(leg as f32 * 72.0, 18.0);
      assert_eq!(covered(&shape, tip.x, tip.y, 0.1), 1.0, "leg {leg}");
      let between = at(leg as f32 * 72.0 + 36.0, 18.0);
      assert_eq!(covered(&shape, between.x, between.y, 0.1), 0.0, "after leg {leg}");
    }
    assert_eq!(covered(&shape, 0.0, 0.0, 0.1), 1.0);
  }

  #[test]
  fn a_star_of_one_leg_is_a_triangle_on_the_origin() {
    let shape = star(1.0);
    assert_eq!(covered(&shape, 0.0, -10.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 3.5, -1.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, 5.0, 0.1), 0.0);
    assert_eq!(covered(&shape, 5.0, -1.0, 0.1), 0.0);
  }

  #[test]
  fn a_stars_sides_bow_out_and_in_but_one_legs_base_stays_straight() {
    // either side of the middle of the one leg's right-hand side, which runs from the tip at (0, -20) to (4, 0)
    let out = Vec2::new(20.0, -4.0).normalize();
    let (beyond, within) = (Vec2::new(2.0, -10.0) + out * 2.0, Vec2::new(2.0, -10.0) - out);
    assert_eq!(covered(&bowed_star(1.0, 0.0), beyond.x, beyond.y, 0.1), 0.0);
    assert_eq!(covered(&bowed_star(1.0, 0.25), beyond.x, beyond.y, 0.1), 1.0);
    assert_eq!(covered(&bowed_star(1.0, 0.0), within.x, within.y, 0.1), 1.0);
    // any further in, and the two sides cross
    assert_eq!(covered(&bowed_star(1.0, -0.07), within.x, within.y, 0.1), 0.0);
    assert_eq!(covered(&bowed_star(1.0, 0.25), 0.0, 1.0, 0.1), 0.0);
  }

  #[test]
  fn a_star_of_two_legs_meets_at_their_bases() {
    let shape = star(2.0);
    for (x, y) in [(3.5, 0.0), (-3.5, 0.0), (0.0, 19.0), (0.0, -19.0)] {
      assert_eq!(covered(&shape, x, y, 0.1), 1.0, "at ({x}, {y})");
    }
    assert_eq!(covered(&shape, 4.5, 0.0, 0.1), 0.0);
  }

  #[test]
  fn a_star_with_no_legs_draws_nothing() {
    let params = Params::new();
    assert_eq!(covered(&star(0.0), 0.0, 0.0, 0.5), 0.0);
    assert_eq!(star(0.0).pose(&env(0.0, &params)).unwrap().outline, vec![]);
    assert_eq!(star(1e9).pose(&env(0.0, &params)).unwrap().outline.len(), MAX_LEGS as usize * 2);
  }

  #[test]
  fn a_chevron_is_a_triangle_with_its_base_cut_in() {
    let shape = Shape::new(ShapeNode::Chevron { width: 20.0.into(), height: 20.0.into(), notch: 0.5.into() });
    for (x, y) in [(0.0, -5.0), (9.0, 8.5), (-9.0, 8.5)] {
      assert_eq!(covered(&shape, x, y, 0.1), 1.0, "at ({x}, {y})");
    }
    for (x, y) in [(0.0, 5.0), (0.0, 9.0), (8.0, -8.0)] {
      assert_eq!(covered(&shape, x, y, 0.1), 0.0, "at ({x}, {y})");
    }
  }

  /// A circle of radius 2 moved to `at`, then `transform`
  fn dot(at: (f32, f32), transform: Transform) -> Shape {
    Shape::new(ShapeNode::circle(2.0)).transform(Transform::Translate(Vec2::from(at).into())).transform(transform)
  }

  fn mirror(center: (f32, f32), rotation: f32) -> Transform {
    Transform::Mirror { center: Vec2::from(center).into(), rotation: rotation.into() }
  }

  fn revolve(center: (f32, f32), angle: f32, count: f32) -> Transform {
    Transform::Revolve { center: Vec2::from(center).into(), angle: angle.into(), count: count.into() }
  }

  fn divide(center: (f32, f32), rotation: f32) -> Transform {
    Transform::Divide { center: Vec2::from(center).into(), rotation: rotation.into() }
  }

  #[test]
  fn a_mirror_draws_the_shape_and_its_reflection() {
    let shape = dot((10.0, 0.0), mirror((0.0, 0.0), 0.0));
    assert_eq!(covered(&shape, 10.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, -10.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, 0.0, 0.1), 0.0);
    // at 90 degrees, the line runs across
    let turned = dot((0.0, 10.0), mirror((0.0, 0.0), 90.0));
    assert_eq!(covered(&turned, 0.0, -10.0, 0.1), 1.0);
    assert_eq!(covered(&turned, -10.0, 0.0, 0.1), 0.0);
    // the line goes through its center
    let moved = dot((10.0, 0.0), mirror((4.0, 0.0), 0.0));
    assert_eq!(covered(&moved, -2.0, 0.0, 0.1), 1.0);
  }

  #[test]
  fn a_revolve_spreads_its_copies_over_its_angle() {
    let shape = dot((10.0, 0.0), revolve((0.0, 0.0), 180.0, 3.0));
    let at = |degrees: f32| Vec2::from_angle(degrees.to_radians()) * 10.0;
    for degrees in [0.0, 60.0, 120.0] {
      assert_eq!(covered(&shape, at(degrees).x, at(degrees).y, 0.1), 1.0, "at {degrees} degrees");
    }
    for degrees in [30.0, 180.0, 240.0] {
      assert_eq!(covered(&shape, at(degrees).x, at(degrees).y, 0.1), 0.0, "at {degrees} degrees");
    }
  }

  #[test]
  fn a_revolve_of_none_draws_nothing() {
    let params = Params::new();
    let shape = dot((10.0, 0.0), revolve((0.0, 0.0), 360.0, 0.0));
    assert_eq!(covered(&shape, 10.0, 0.0, 0.1), 0.0);
    assert_eq!(shape.outline(&env(0.0, &params)), Vec::<Vec<Vec2>>::new());
  }

  #[test]
  fn a_divide_keeps_one_side_of_its_line() {
    let shape = Shape::new(ShapeNode::circle(10.0)).transform(divide((0.0, 0.0), 0.0));
    assert_eq!(covered(&shape, 5.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, -5.0, 0.0, 0.1), 0.0);
    assert!((covered(&shape, 0.0, 0.0, 0.5) - 0.5).abs() < 1e-4);
    // at 90 degrees the bottom is kept, and the edge of the circle still bounds it
    let turned = Shape::new(ShapeNode::circle(10.0)).transform(divide((0.0, 0.0), 90.0));
    assert_eq!(covered(&turned, 0.0, 5.0, 0.1), 1.0);
    assert_eq!(covered(&turned, 0.0, -5.0, 0.1), 0.0);
    assert_eq!(covered(&turned, 0.0, 11.0, 0.1), 0.0);
  }

  #[test]
  fn transforms_after_a_copy_or_cut_move_all_of_it() {
    let copies = dot((10.0, 0.0), revolve((0.0, 0.0), 360.0, 2.0)).transform(Transform::Translate(Vec2::new(100.0, 0.0).into()));
    assert_eq!(covered(&copies, 110.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&copies, 90.0, 0.0, 0.1), 1.0);

    let cut = Shape::new(ShapeNode::circle(10.0)).transform(divide((0.0, 0.0), 0.0)).transform(Transform::Scale(2.0.into()));
    assert_eq!(covered(&cut, 15.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&cut, -15.0, 0.0, 0.1), 0.0);
    // still in scene units across the cut, however it's scaled
    assert!((covered(&cut, 0.25, 0.0, 0.5) - 0.75).abs() < 1e-4);
  }

  #[test]
  fn a_divide_after_a_revolve_cuts_every_copy() {
    let shape = dot((10.0, 0.0), revolve((0.0, 0.0), 360.0, 2.0)).transform(divide((10.0, 0.0), 0.0));
    assert_eq!(covered(&shape, 11.0, 0.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 9.0, 0.0, 0.1), 0.0);
    assert_eq!(covered(&shape, -10.0, 0.0, 0.1), 0.0);
  }

  #[test]
  fn a_rotate_after_a_revolve_turns_around_its_center() {
    let shape = dot((10.0, 0.0), revolve((0.0, 0.0), 360.0, 2.0)).transform(Transform::Rotate(90.0.into()));
    assert_eq!(covered(&shape, 0.0, 10.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 0.0, -10.0, 0.1), 1.0);
    assert_eq!(covered(&shape, 10.0, 0.0, 0.1), 0.0);
  }

  #[test]
  fn a_pose_has_where_each_transform_turns_around() {
    let params = Params::new();
    let pose = dot((10.0, 5.0), mirror((0.0, 0.0), 0.0)).pose(&env(0.0, &params)).unwrap();
    // into the translate, into the mirror, then on its line
    let expected = [Vec2::ZERO, Vec2::new(10.0, 5.0), Vec2::new(0.0, 5.0)];
    assert_eq!(pose.pivots.len(), expected.len());
    assert!(pose.pivots.iter().zip(expected).all(|(pivot, expected)| pivot.abs_diff_eq(expected, 1e-4)), "{:?}", pose.pivots);
    assert_eq!(pose.transforms[1], Affine2::IDENTITY);
    assert_eq!(pose.centers, vec![None, Some(Vec2::ZERO)]);
    assert!(pose.outline.iter().all(|point| point.x > 0.0), "the shape itself, without its reflection");
  }

  #[test]
  fn a_sample_takes_the_local_position_of_the_copy_it_is_in() {
    let params = Params::new();
    let shape = dot((10.0, 0.0), revolve((0.0, 0.0), 360.0, 2.0)).prepare(&env(0.0, &params)).unwrap();
    let local = shape.cover(&Sample::new(Vec2::new(-10.0, 1.0), 0.1)).unwrap().local;
    assert!(local.abs_diff_eq(Vec2::new(0.0, -1.0), 1e-4));
  }

  #[test]
  fn an_outline_has_a_loop_for_each_copy_with_cuts_taken_off() {
    let params = Params::new();
    let frame = env(0.0, &params);
    assert_eq!(dot((10.0, 0.0), revolve((0.0, 0.0), 180.0, 3.0)).outline(&frame).len(), 3);
    let cut = Shape::new(ShapeNode::circle(10.0)).transform(divide((0.0, 0.0), 0.0)).outline(&frame);
    assert_eq!(cut.len(), 1);
    assert!(cut[0].iter().all(|point| point.x >= -1e-4));
    assert!(cut[0].iter().any(|point| point.abs_diff_eq(Vec2::new(0.0, 10.0), 1e-3)));
  }

  fn polyline(points: &[(f32, f32)]) -> ShapeNode {
    ShapeNode::Polyline { points: points.iter().map(|(x, y)| Vec2::new(*x, *y).into()).collect() }
  }

  fn bezier(points: &[((f32, f32), (f32, f32))]) -> ShapeNode {
    let points = points.iter().map(|(point, handle)| BezierPoint { point: Vec2::from(*point).into(), handle: Vec2::from(*handle).into() });
    ShapeNode::Bezier { points: points.collect() }
  }

  #[test]
  fn a_polyline_goes_from_its_first_point_to_its_last_at_an_even_speed() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = Shape::new(polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0)]));
    for (phase, x, y) in [(0.0, 0.0, 0.0), (0.5, 20.0, 0.0), (0.75, 30.0, 0.0), (1.0, 30.0, 10.0)] {
      assert!(path.along(phase, &frame).abs_diff_eq(Vec2::new(x, y), 1e-4), "at {phase}");
    }
  }

  #[test]
  fn a_polyline_of_one_point_stays_on_it_and_one_of_none_at_the_origin() {
    let params = Params::new();
    let frame = env(0.0, &params);
    assert_eq!(Shape::new(polyline(&[(3.0, 4.0)])).along(0.5, &frame), Vec2::new(3.0, 4.0));
    assert_eq!(Shape::new(polyline(&[])).along(0.5, &frame), Vec2::ZERO);
  }

  #[test]
  fn a_line_has_no_inside_to_draw() {
    let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
    assert_eq!(covered(&Shape::new(polyline(&square)), 5.0, 5.0, 0.5), 0.0);
    assert_eq!(covered(&Shape::new(polyline(&square)), 5.0, 0.0, 0.5), 0.0);
    let curve = bezier(&[((0.0, 0.0), (0.0, 10.0)), ((10.0, 0.0), (0.0, -10.0))]);
    assert_eq!(covered(&Shape::new(curve), 5.0, 5.0, 0.5), 0.0);
  }

  #[test]
  fn a_line_does_not_close() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let pose = Shape::new(polyline(&[(0.0, 0.0), (30.0, 0.0), (30.0, 10.0)])).pose(&frame).unwrap();
    assert_eq!(pose.outline.len(), 3);
    assert_eq!(pose.points, pose.outline);
    assert_eq!(pose.bounds, Bounds::new(Vec2::ZERO, Vec2::new(30.0, 10.0)));
    assert_eq!(pose.handles, vec![]);
  }

  #[test]
  fn a_bezier_with_no_handles_is_a_polyline() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = Shape::new(bezier(&[((0.0, 0.0), (0.0, 0.0)), ((30.0, 0.0), (0.0, 0.0)), ((30.0, 10.0), (0.0, 0.0))]));
    assert!(path.along(0.5, &frame).abs_diff_eq(Vec2::new(20.0, 0.0), 1e-3));
  }

  #[test]
  fn a_bezier_goes_through_its_points_and_leans_toward_its_handles() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = Shape::new(bezier(&[((0.0, 0.0), (0.0, 12.0)), ((20.0, 0.0), (0.0, -12.0))]));
    assert_eq!(path.along(0.0, &frame), Vec2::ZERO);
    assert!(path.along(1.0, &frame).abs_diff_eq(Vec2::new(20.0, 0.0), 1e-4));
    // the same arch both ways, and 3/4 of the way out to the handles at its top
    assert!(path.along(0.5, &frame).abs_diff_eq(Vec2::new(10.0, 9.0), 1e-2));
  }

  #[test]
  fn a_handle_stays_with_its_point_and_arrives_from_the_opposite_way() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let path = ShapeNode::Bezier {
      points: vec![
        BezierPoint { point: Vec2::ZERO.into(), handle: Vec2::ZERO.into() },
        BezierPoint { point: Vec2::new(10.0, 0.0).into(), handle: Vec2::new(0.0, 5.0).into() },
        BezierPoint { point: Vec2::new(20.0, 0.0).into(), handle: Vec2::ZERO.into() },
      ],
    };
    let line = path.outline(&frame);
    let middle = line[BEZIER_STEPS];
    assert_eq!(middle, Vec2::new(10.0, 0.0));
    let (before, after) = (line[BEZIER_STEPS - 1] - middle, line[BEZIER_STEPS + 1] - middle);
    assert!(before.normalize().dot(after.normalize()) < -0.999, "turns a corner at the point");
    assert!(after.y > 0.0 && before.y < 0.0);
  }

  #[test]
  fn a_beziers_middle_is_that_of_its_points() {
    let params = Params::new();
    let frame = env(0.0, &params);
    let pose = Shape::new(bezier(&[((0.0, 0.0), (0.0, 100.0)), ((20.0, 10.0), (0.0, -100.0))])).pose(&frame).unwrap();
    assert_eq!(pose.bounds, Bounds::new(Vec2::ZERO, Vec2::new(20.0, 10.0)));
    assert_eq!(pose.points, vec![Vec2::ZERO, Vec2::new(20.0, 10.0)]);
    assert_eq!(pose.handles, vec![Vec2::new(0.0, 100.0), Vec2::new(0.0, -100.0)]);
    assert_eq!(pose.outline.len(), 1 + BEZIER_STEPS);
  }

  #[test]
  fn a_bezier_is_animated_by_a_point_or_a_handle() {
    let moves = || Prop::Keyframes(Keyframes { keys: vec![Key::new(0.0, Vec2::ZERO, Curve::Linear)], cycle: Cycle::Once });
    let by_point = BezierPoint { point: moves(), handle: Vec2::ZERO.into() };
    let by_handle = BezierPoint { point: Vec2::ZERO.into(), handle: moves() };
    assert!(ShapeNode::Bezier { points: vec![by_point] }.is_animated());
    assert!(ShapeNode::Bezier { points: vec![by_handle] }.is_animated());
    assert!(!bezier(&[((0.0, 0.0), (1.0, 1.0))]).is_animated());
  }

  #[test]
  fn a_rect_is_its_width_and_height_around_the_origin() {
    let rect = Shape::new(ShapeNode::Rect { width: 10.0.into(), height: 4.0.into() });
    assert_eq!(covered(&rect, 0.0, 0.0, 0.5), 1.0);
    assert_eq!(covered(&rect, -4.0, 1.5, 0.5), 1.0);
    assert_eq!(covered(&rect, 0.0, 3.0, 0.5), 0.0);
    assert!((covered(&rect, 5.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&rect, 0.0, -2.0, 0.5) - 0.5).abs() < 1e-5);
  }

  #[test]
  fn a_rect_edge_is_in_scene_units_when_scaled() {
    let rect = Shape::new(ShapeNode::Rect { width: 1.0.into(), height: 0.2.into() }).transform(Transform::Scale(20.0.into()));
    assert!((covered(&rect, 10.0, 0.0, 0.5) - 0.5).abs() < 1e-5);
    assert!((covered(&rect, 10.25, 0.0, 0.5) - 0.25).abs() < 1e-5);
    assert!((covered(&rect, 0.0, 2.25, 0.5) - 0.25).abs() < 1e-5);
  }

  #[test]
  fn a_rect_past_a_corner_is_measured_to_the_corner() {
    let rect = Shape::new(ShapeNode::Rect { width: 2.0.into(), height: 2.0.into() });
    assert_eq!(covered(&rect, 1.3, 1.4, 0.2), 0.0);
    assert!(covered(&rect, 1.05, 1.05, 0.2) > 0.0);
  }

  #[test]
  fn a_rect_with_no_width_draws_nothing() {
    let rect = Shape::new(ShapeNode::Rect { width: 0.0.into(), height: 4.0.into() });
    assert_eq!(covered(&rect, 0.0, 0.0, 0.5), 0.0);
  }

  #[test]
  fn sizes_change_over_time() {
    let params = Params::new();
    let grows = Prop::Keyframes(Keyframes {
      keys: vec![Key::new(0.0, 2.0, Curve::Linear), Key::new(1.0, 6.0, Curve::Linear)],
      cycle: Cycle::Once,
    });
    let rect = Shape::new(ShapeNode::Rect { width: grows, height: 4.0.into() });
    assert!(rect.is_animated());
    assert_eq!(rect.base.bounds(&env(0.5, &params)), Bounds::new(Vec2::new(-2.0, -2.0), Vec2::new(2.0, 2.0)));
  }

  #[test]
  fn a_pose_has_what_an_editor_needs_to_work_out_a_drag() {
    let params = Params::new();
    let triangle = ShapeNode::Polygon {
      points: vec![Vec2::new(0.0, 0.0).into(), Vec2::new(4.0, 0.0).into(), Vec2::new(0.0, 2.0).into()],
    };
    let shape = Shape::new(triangle).transform(Transform::Translate(Vec2::new(1.0, 2.0).into()));
    let pose = shape.pose(&env(0.0, &params)).unwrap();
    assert_eq!(pose.transforms, vec![Affine2::from_translation(Vec2::new(1.0, 2.0))]);
    assert_eq!(pose.bounds, Bounds::new(Vec2::ZERO, Vec2::new(4.0, 2.0)));
    assert_eq!(pose.points.len(), 3);
  }

  #[test]
  fn a_pose_turns_around_the_shape_where_it_has_been_moved() {
    let params = Params::new();
    let shape = Shape::new(ShapeNode::circle(1.0))
      .transform(Transform::Translate(Vec2::new(10.0, 0.0).into()))
      .transform(Transform::Rotate(90.0.into()));
    let pose = shape.pose(&env(0.0, &params)).unwrap();
    assert!(pose.transforms[1].transform_point2(Vec2::new(10.0, 0.0)).abs_diff_eq(Vec2::new(10.0, 0.0), 1e-5));
  }

  #[test]
  fn a_collapsed_shape_has_no_pose() {
    let params = Params::new();
    let flat = Shape::new(ShapeNode::circle(1.0)).transform(Transform::Scale(0.0.into()));
    assert_eq!(flat.pose(&env(0.0, &params)), None);
  }

  #[test]
  fn zero_radius_is_a_hard_edge() {
    let circle = Shape::new(ShapeNode::circle(0.5)).transform(Transform::Scale(10.0.into()));
    assert_eq!(covered(&circle, 4.99, 0.0, 0.0), 1.0);
    assert_eq!(covered(&circle, 5.01, 0.0, 0.0), 0.0);
  }
}
