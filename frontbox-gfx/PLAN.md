# frontbox-gfx Plan

One library for describing graphics and how they move over time, for a list of LEDs, LEDs on a playfield plane, or a
DMD. There is one model for all of them: a scene is drawn on a 2d stage, and a list of LEDs is laid out on that stage
as a line.

The rules this plan settles on are kept in [AGENTS.md](AGENTS.md). This file is the roadmap.

## Approach

Start small. The first goal is solid colors and gradients, on a list of LEDs and in filled shapes on a 2d stage. Each
later feature is added when a scene needs it, and is shaped as a general piece that combines with what's already
there, not as a one-off option.

New types are built alongside the existing `color_sequence`, `canvas`, and `animation` modules, without changing them.
Once the new types cover what the old ones do, users are migrated and the old modules are deleted.

## Sampling

Everything is a function that's sampled, not a buffer that's filled. A device decides where its points are and asks
the scene for a color at each one:

```rust
/// A position in stage units and the radius of the area it covers
pub struct Sample { pub pos: Vec2, pub radius: f32 }

/// What's the same for every sample in one frame
pub struct Frame<'a> { pub time: f32, pub stage: Bounds, pub params: &'a Params }

pub fn sample(&self, at: &Sample, frame: &Frame) -> Rgba<u8>;
```

`radius` is the footprint of what's being lit: half the pixel pitch on a DMD, the size of an insert on the playfield,
half the spacing for an LED on a line. Shape edges use it for coverage, so an edge passing through an LED lights it
partially.

Sampling reads the definition directly, resolving each `Prop` at the given time. There's no separate "resolved frame"
type mirroring the definition. If that's ever too slow, caching is added then.

### The stage

Scenes are drawn on a **stage**, a rectangle in stage units:

- **Sized** (`size: Some((w, h))`) -- The stage is in real units (the units LED locations already use). A DMD or the
  playfield maps it onto itself.
- **Unsized** (`size: None`) -- The stage is the target's, passed in the `Frame`.

Where a device's points sit is the device's business, kept apart from the scene types in the `layout` module. A
`layout::Line` of `n` points (a strip of LEDs) puts point `i` at `(i, 0)` with a radius of 0.5, and its stage runs from
the first point's center to the last's. So a gradient across the stage puts its end colors exactly on the end LEDs (as
`ColorSequence` does), and positions on a line are LED indexes.

## Definition types

Sketches, not final signatures.

### Values that change: `Value`, `Prop`, `Track`, `Key`

Parameters are values the game supplies at runtime, typed so they can be positions and colors as well as numbers:

```rust
pub enum Value {
  Float(f32),
  Vec2(Vec2),
  Color(Color),
}

/// `Rgba<u8>` has no serde support (image's `serde` feature doesn't cover pixel types), so definitions hold this.
/// Written as `(r, g, b, a)`. Sampling returns plain `Rgba<u8>`.
pub struct Color(pub Rgba<u8>);
```

Every field that could change is a `Prop<T>`, so any of them can be fixed, read from a parameter, or keyframed:

```rust
pub enum Prop<T> {
  /// A value that doesn't change
  Fixed(T),
  /// Read from a parameter
  Param(String),
  /// Keyframes over time
  Track(Track<T>),
}

pub struct Track<T> {
  pub keys: Vec<Key<T>>,
  pub cycle: Cycle,          // Once | Times(n) | Forever, from `animation`
}

pub struct Key<T> {
  pub at: f32,               // seconds
  pub value: T,
  pub curve: Curve,          // how to get from this key to the next, from `animation`
}
```

In RON: `radius: Fixed(4.0)`, `radius: Param("size")`, or `radius: Track(keys: [...])`. A parameter that's missing
or of the wrong type resolves to the type's default. A looping track ends at its last key and jumps back to its first.

### Nodes: warp, kind, adjust

Shapes and paints are both functions of position: a shape gives a distance to its edge, a paint gives a color.
Anything that changes one of them either changes what goes in or what comes out, so both are the same pipeline:

```rust
pub struct Node<K, A> {
  pub warp: Vec<Warp>,   // changes the position going in
  pub kind: K,           // the base function
  pub adjust: Vec<A>,    // changes the output coming out
}

pub type Shape = Node<ShapeNode, ShapeAdjust>;
pub type Paint = Node<PaintNode, ColorAdjust>;
```

- **Warps** don't depend on what they wrap, so there's one `Warp` enum for every node. Warps are listed in the order
  they're applied to the drawing (scale, then rotate, then move); sampling undoes them in reverse on the sample point.
- **Adjusts** do depend on the output: a distance and a color need different operations, so each output type has its
  own adjust enum.
- Both lists default to empty, so a node only states its kind unless it changes something.

Modifiers are items in these lists, not new fields or wrapper variants. Combining two things (layers, masks, shape
unions) is structural and isn't a modifier.

```rust
pub enum Warp {
  Translate(Prop<Vec2>),
  Rotate(Prop<f32>),     // degrees, clockwise on a stage whose y runs down
  Scale(Prop<Vec2>),
}

pub enum ShapeAdjust {}  // none yet

pub enum ColorAdjust {
  HueShift(Prop<f32>),   // degrees, keeping saturation, lightness, and alpha
}
```

### Shape kinds

```rust
pub enum ShapeNode {
  /// An ellipse 1 unit across, centered on the origin
  Ellipse,
  /// A closed polygon through points
  Polygon { points: Vec<Prop<Vec2>> },
}
```

Shapes are signed distance functions. Coverage at a sample comes from the distance to the edge and the sample's
radius (a linear ramp across the footprint, so a sample centered on the edge is half lit), which gives smooth edges on
sparse inserts and dense DMDs alike. Distances are measured in the shape's own space and converted to stage units
through its warps, which is exact under uniform scale and close to the edge under stretching.

Shapes have no position or size of their own; warps give them that. A circle of radius 20 is an `Ellipse` scaled by
`(40, 40)`, and a rect is a unit square polygon scaled to its size. `Rect` and `Circle` can come back later as sugar.

### Paint kinds

```rust
pub enum PaintNode {
  Solid(Prop<Color>),
  /// Runs left to right across the paint's space
  Gradient { stops: Vec<Stop>, space: ColorSpace },  // Oklab (default) | Rgb
}

pub struct Stop { pub at: f32, pub color: Prop<Color> }
```

A paint is drawn in its own space, a unit box from `-0.5` to `0.5`, and a gradient's `t` is `x + 0.5`. The layer
stretches that box over its shape's own bounds (before the shape's warps), and then the shape's warps apply to both.
So a gradient fills its shape and moves and turns with it. With no shape, the box is the stage, so on a line of LEDs
it runs from the first LED to the last.

- A vertical or diagonal gradient is a gradient with a `Rotate` warp; there's no `angle`. Past the ends, `t` clamps to
  the end colors.
- To keep a gradient still while its shape turns, give the paint the opposite rotation.
- Gradients interpolate in Oklab by default, so brightness stays even. In RGB, red to blue dips to a dark `#800080`
  in the middle, which is very visible on LEDs.

### Layer, scene

```rust
pub struct Layer {
  pub paint: Paint,
  pub shape: Option<Shape>,  // None covers the whole stage
}

/// The whole definition, and what a file holds
pub struct Scene {
  pub version: u32,
  pub params: Params,        // the inputs the game can set: name and default
  pub size: Option<Vec2>,    // the stage in real units, or None to take the target's size
  pub layers: Vec<Layer>,    // painted bottom to top, alpha over
}
```

`params` is the scene's list of inputs, like a function signature. It gives each parameter a default, so a scene
still draws when the game doesn't set one, and it says what an editor or the game can change without searching the
layers for `Param(...)`.

"Effect" isn't used as a type name, since it suggests a filter applied to something else (a blur). It stays free in
case a concept like that shows up.

Fields that aren't written take their default (`#[serde(default)]`). RON's `implicit_some` and
`unwrap_variant_newtypes` extensions are on, so a file only states what it changes and avoids double parentheses.
That includes tuples inside a variant: a color is `Fixed(255, 51, 0, 255)` and a size is `Fixed(40.0, 40.0)`; the
doubled form `Fixed((40.0, 40.0))` doesn't read. Hex colors may come back later as sugar.

## Examples

A red-to-blue gradient along a strip:

```ron
(
  version: 1,
  layers: [
    (paint: (kind: Gradient(stops: [
      (at: 0.0, color: Fixed(255, 0, 0, 255)),
      (at: 1.0, color: Fixed(0, 0, 255, 255)),
    ]))),
  ],
)
```

A circle with a vertical gradient over a solid background, on a sized stage:

```ron
(
  version: 1,
  size: (100.0, 50.0),
  layers: [
    (paint: (kind: Solid(Fixed(16, 16, 16, 255)))),
    (
      shape: (
        kind: Ellipse,
        warp: [Scale(Fixed(40.0, 40.0)), Translate(Fixed(50.0, 25.0))],
      ),
      paint: (
        kind: Gradient(stops: [
          (at: 0.0, color: Fixed(255, 204, 0, 255)),
          (at: 1.0, color: Fixed(255, 51, 0, 255)),
        ]),
        warp: [Rotate(Fixed(90.0))],
      ),
    ),
  ],
)
```

Sent to LEDs at their positions on the playfield, the first file sweeps red to blue across it.

## First goal

Done. The new types are in `src/scene`, and frontbox-led plays them with `LedScene`.

- [x] Add `serde`, `glam`, and `ron` dependencies
- [x] `file`: RON load/save with `implicit_some` and `unwrap_variant_newtypes`, `version` checking (a newer file
      is reported as newer, not as a parse error)
- [x] `Sample`, `Frame`, `Bounds`, and `layout::Line` for laying out `n` points (a strip of LEDs) on an unsized stage
- [x] `Value`, `Color`, `Prop`, `Track`, `Key`, resolved from a time and parameters, reusing `Curve`, `Cycle`, and
      `Lerp` (`Curve` and `Cycle` now derive serde; `Curve::Custom` was dropped)
- [x] `Node`, and `Warp` with translate, rotate, and scale
- [x] `ShapeNode::Ellipse` and `ShapeNode::Polygon`, with coverage from the sample radius
- [x] `PaintNode::Solid` and `PaintNode::Gradient` (Oklab and RGB), drawn in the shape's own space
- [x] `Layer` and `Scene`, sampled with alpha-over compositing
- [x] Tests: a gradient on a line against `ColorSequence`; shapes sampled on a grid (DMD-like) and at sparse points
      (playfield-like); a gradient turning with its shape; RON round-trips that sample the same
- [x] frontbox-led: `LedScene` plays a scene on an `LedQ` as a line
- [x] One color adjust, `HueShift`, to prove the adjust list

## Set aside for now

Ideas from earlier drafts, to pick up when a scene needs them. Most now land as an item in an existing list rather
than a new type.

- More warps: displace, mirror, repeat; time warps (offset, rate, stutter), which is where a nested scene's timing
  would live
- Shape adjusts: round, inflate, outline (strokes), softness
- More color adjusts: opacity, brightness, dropout, posterize
- Combining: blend modes, masks, shape union and subtract, groups of layers
- Gradient extras: patterns and shuffle; more ways to get `t` (radial, conic, along an outline). Reverse may just be
  `Scale((-1, 1))`, and a wrapping offset may be repeat plus translate
- More kinds: noise, sparkle, images, nested scenes; bands, paths, growth
- Playback: an `Instance` with play/stop, outros on stop, rate, tracks driven by a parameter instead of time,
  completion
- Parameter scopes (per instance and global), relative lengths (`Extent`), stage-to-device fitting (`Viewport`)
- Placing a list of LEDs at its positions on a plane, and a DMD adapter
- Sugar: a bare kind for a node with no warps or adjusts (`paint: Solid(...)`), `Rect` and `Circle`, evenly spaced
  gradient colors without writing `at`, hex colors

## Open questions

- **Random props.** A `Prop::Random` for values picked per instance. Needs deciding when it rerolls and how instances
  get a seed.
