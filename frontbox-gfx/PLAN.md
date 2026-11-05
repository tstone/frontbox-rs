# frontbox-gfx Plan

One library for describing graphics in 1d (an ordered list of LEDs) and 2d (a DMD, or LEDs on a playfield plane), and
how those graphics move over time. 3d is out of scope for now, but nothing here should rule it out.

The rules this plan settles on are kept in [AGENTS.md](AGENTS.md). This file is the roadmap for getting there.

## Approach

New types are built alongside the existing `color_sequence`, `canvas`, and `animation` modules, without changing them.
Once the new types cover what the old ones do, users are migrated and the old modules are deleted. Until then, both
compile side by side, and the old modules only get additive changes (like a `serde` derive on `Curve`).

## Architecture

Four layers, of which gfx owns the first three:

1. **Definition** -- What an effect looks like and how it moves. Plain data (enums and structs), serializable to RON.
   Immutable once loaded.
2. **Playback** -- An instance of a definition: elapsed time, cycle count, active state, and current parameter values.
   This is the definition/instance split Flash had between symbols and their instances.
3. **Sampling** -- Asking a resolved effect for the color at a point. 1d samples take a `t` along a line, 2d samples
   take a position with a footprint. gfx never allocates a buffer for a device.
4. **Device adapters** (outside gfx) -- `frontbox-led` and `frontbox-dmd` decide where the sample points are and what
   to do with the colors.

```text
Effect (definition, RON) ──▶ Instance (playback) ──resolve(time, params)──▶ Frame ──sample(points)──▶ colors
                                                                                      ▲
                                             frontbox-led / frontbox-dmd provide the points and consume the colors
```

### Sampling

Everything is a function that's sampled, not a buffer that's filled.

```rust
/// A 1d sample: where along the line, and how long the line is in its own units
pub struct Span { pub length: f32 }
pub fn sample_1d(&self, t: f32, span: &Span) -> Rgba<u8>;

/// A 2d sample: a position in stage units and the radius of the area it covers
pub struct Sample { pub pos: Vec2, pub radius: f32 }
pub fn sample_2d(&self, sample: &Sample) -> Rgba<u8>;
```

- `Span::length` resolves `Extent::Absolute` values and pattern cell sizes.
- `Sample::radius` is the footprint of what's being lit: half the pixel pitch on a DMD, the size of the insert or LED
  on a playfield. Shapes use it for coverage, so a thin line passing between two inserts still lights both partially
  instead of neither. This replaces the idea of a global DPI.
- Per-point randomness (sparkle, dropout) hashes the sample position, so it's stable from frame to frame without gfx
  knowing which LED it is.

### What sets the 1d spacing

A 1d paint only sees `t`. The thing it's applied to decides what a unit of length is, so there's no spacing setting:

- **A list of LEDs** -- `t = i / (n - 1)` and `Span::length = n`. One unit is one LED: "red, white, repeat" alternates
  LEDs.
- **A path in 2d** -- `t` is the distance along the path divided by its total length, and `Span::length` is that total
  in real units. Whatever inserts are under the path take the color at their position. Evenly spaced color over
  unevenly spaced LEDs is a path drawn through them.

### Coordinates and units

2d effects are authored on a **stage**: a rectangle with a size in real units (the same units LED locations already
use). An adapter maps the stage onto its device. For a DMD that's a pixel grid with a pitch, for the playfield it's a
rectangle on a `ReferencePlane`. gfx provides the mapping math (`Viewport`, with stretch/contain/cover fits), since it's
pure geometry; the adapter supplies the device side.

Coordinates are `f32` (`glam::Vec2`). Extents in new code are `Extent<f32>`.

## Definition types

Sketches, not final signatures.

### Parameters

Parameters are values the game supplies at runtime. They're typed, since effects need positions ("from the point the
ball hit") and colors (a district's accent), not just numbers:

```rust
pub enum Value {
  Float(f32),
  Vec2(Vec2),
  Color(Rgba<u8>),
}
```

An effect declares its parameters with defaults. Each one has a scope:

- **Instance** -- set on one playing instance, e.g. a wave's origin or a progress percent.
- **Global** -- shared by every instance, e.g. the music's beat position, or Steward integrity (0--6). gfx defines
  the scope; the owner that sets global values lives outside gfx.

### Properties

Every value that can change over time is a `Prop<T>` instead of a bare `T`. This is how both keyframes and parameters
work, without needing paths to address properties:

```rust
pub enum Prop<T> {
  /// A value that doesn't change
  Fixed(T),
  /// Read from a parameter
  Param(String),
  /// Keyframes over a timeline
  Track(Track<T>),
}

pub struct Track<T> {
  /// What moves the track forward: time, or a parameter (a hit count, or the global beat position for tempo sync)
  pub driver: Driver,
  pub keys: Vec<Key<T>>,
  pub cycle: Cycle,
}

pub struct Key<T> {
  pub at: f32,     // seconds when driven by time, parameter value otherwise
  pub value: T,
  pub curve: Curve, // how to get from this key to the next
}
```

In RON this reads naturally: `rotation: Fixed(45.0)`, `rotation: Param("spin")`, or `rotation: Track((keys: [...]))`.
What `Sequence` and `Delay` do today becomes keys on a track: a delay is a first key at a later time, a sequence is
more keys. Centers, path points, window lengths, and colors are all `Prop`s, so any of them can come from a parameter.

### Timeline

```rust
pub struct Timeline {
  pub intro: Option<f32>,  // seconds; plays once on start
  pub loop_: Option<f32>,  // seconds; repeats until stopped
  pub outro: Option<f32>,  // seconds; plays on stop, then the effect clears
  pub rate: Prop<f32>,     // playback speed, e.g. a heartbeat that speeds up with each hit
}
```

Segments are ranges of the same time axis that every track reads. `stop()` jumps to the outro instead of cutting off,
which covers "the shot leaving drains away" and "forest effects pull back into the roots". An effect without segments
plays through once and follows its `end` behavior.

### 1d

```rust
pub struct Paint1d {
  pub fill: Fill1d,       // Solid | Gradient { stops } | Pattern { colors, cycle, cell }
  pub window: Window1d,   // Full | Padded { start, end } | Anchored { anchor, length }
  pub offset: Prop<f32>,  // rotation along the line, wraps; continuous, so it can move by less than one LED
  pub reverse: bool,
  pub shuffle: Option<u64>,
}

pub struct Layer1d {
  pub paint: Paint1d,
  pub opacity: Prop<f32>,
  pub blend: Blend,
}

pub struct Scene1d {
  pub layers: Vec<Layer1d>, // composited bottom to top
}
```

Most of `ColorSequence`'s alterations become changes to `t`: reverse is `1 - t`, rotate is `(t + offset) mod 1`, the
window maps `t` into a sub-range. Shuffle and pattern are discrete and use `Span::length` to find their cells.

Layers replace `ColorSequence::overwrite`. A hex ring with filled segments and a flickering last segment is a window
layer plus a one-LED layer with its own flicker track.

### 2d

```rust
pub enum Paint2d {
  Solid(Prop<Rgba<u8>>),
  /// A 1d paint along an axis
  Linear { paint: Paint1d, angle: Prop<f32> },
  /// A 1d paint outward from a center
  Radial { paint: Paint1d, center: Prop<Vec2>, radius: Prop<f32> },
  /// A 1d paint around a center
  Conic { paint: Paint1d, center: Prop<Vec2> },
  /// Animated simplex noise, used as `t` into a 1d paint
  Noise { paint: Paint1d, scale: Prop<f32>, speed: Prop<f32>, seed: u64 },
  /// Random points that flicker on and off
  Sparkle { color: Prop<Rgba<u8>>, density: Prop<f32>, rate: Prop<f32>, seed: u64 },
  Image { asset: String, sampling: ImageSampling },
}

/// Shapes are signed distance functions, which gives soft edges, strokes, and glow from one value
pub enum Shape {
  Fill,                          // the whole stage
  Circle { radius: Prop<f32> },
  Rect { size: Prop<Vec2>, corner_radius: Prop<f32> },
  Ring { radius: Prop<f32>, width: Prop<f32> },
  Polygon { points: Vec<Prop<Vec2>> },
  Path { points: Vec<Prop<Vec2>>, width: Prop<f32> },
  Growth(Growth),                // see below
}

pub struct Layer2d {
  pub content: Content2d,        // Shape { shape, fill, stroke } | Effect(EffectRef), see Composition
  pub transform: Transform,      // translate, rotate, scale: all Prop
  pub opacity: Prop<f32>,
  pub softness: Prop<f32>,       // edge falloff beyond the sample footprint
  pub displace: Option<Displace>,// noise or sine offset on the shape's edge, e.g. a fill's moving surface
  pub blend: Blend,
  pub mask: Option<Mask>,
  pub filters: Vec<Filter>,
}

pub struct Scene2d {
  pub size: Vec2,                // the stage, in real units
  pub layers: Vec<Layer2d>,      // composited bottom to top
}
```

`Path` with a `Paint1d` stroke is the bridge between 1d and 2d: the same paint can go along a list of LEDs, or along a
path on the playfield or DMD, where `t` comes from the nearest point on the path.

### Growth

Roots, lightning, and vines spread from an origin toward a target. Rather than walking a graph of the real LEDs (which
gfx can't know about), growth generates branching paths as geometry on the stage, and the inserts under them light up:

```rust
pub struct Growth {
  pub origin: Prop<Vec2>,
  pub target: Prop<Vec2>,
  pub seed: u64,
  pub density: Prop<f32>,     // how many branches, and how closely they fill the area between origin and target
  pub spread: Prop<f32>,      // how far branches wander from the line between origin and target
  pub jitter: Prop<f32>,      // how crooked each branch is
  pub width: Prop<f32>,
  pub progress: Prop<f32>,    // 0..1, how far the growth has reached; tween it down to pull back
  pub trail: Paint1d,         // color by age behind the growing tip: fades out, or stays solid
}
```

- The same seed always generates the same branches. Geometry is regenerated only when origin, target, seed, density,
  spread, or jitter change, and is cached in the resolved frame.
- The target makes the growth directional, so it overlaps inserts well on most playfields. `density` is tuned per
  effect until it looks right for the inserts it crosses. That's an approximation, and it's good enough.
- `progress` reveals each branch along its length. `t` along a branch, relative to the tip, picks the color from
  `trail`.

### Composition

Effects are built from other effects:

- **Nested effects** -- `Content2d::Effect` (and its 1d counterpart) places another effect by name, with its own time
  offset, rate, and parameter values. Changing its timing is how stutter works: hold its time for random lengths.
- **Masks** -- A layer can be limited to a shape, or to another layer's alpha, optionally inverted. The takeover motif
  is a city effect under a forest effect, with the forest masked by a growing shape or growth.
- **Blend modes** -- `Normal` (alpha over), `Add` (glow), `Multiply` (brightness, e.g. noise darkening another layer),
  `Screen`. gfx provides the blend math, so `LedSystem` can use the same functions when compositing effects from
  different systems.
- **Filters** -- Applied to a layer's output: `Dropout { amount, seed }`, `HueShift`, `Brightness`, `Posterize`.
  Glitch is dropout, hue shift, and stutter together.

Nested effects are looked up by name in a `Library` loaded from a directory of effect files. References that form a
loop are an error at load time.

### The effect file

```rust
pub struct Effect {
  pub version: u32,
  pub params: BTreeMap<String, ParamDecl>, // type, scope, default
  pub timeline: Timeline,
  pub end: EndBehavior,                    // Hold | Clear
  pub body: Body,                          // OneD(Scene1d) | TwoD(Scene2d)
}
```

Colors are `Rgba<u8>` in memory. In RON they're hex strings: `"#ff3300"` (opaque) or `"#ff330080"` (with alpha).

## Litmus test: lotko's effect primitives

Last of the Kilmore Oaks (`lotko-homebrew/RULES.md`) lists the primitives its effects are built from. How each maps:

| Primitive | Built from | Phase |
|---|---|---|
| Radial wave | `Ring` with a radius track, `softness` for falloff, center from a parameter | 3 |
| Plane sweep | Replaced for now by a `Rect` whose position is tweened across the stage. A true 3d sweep into the backbox is deferred | 3 |
| Growth | `Shape::Growth` | 4 |
| Path flow | A `Path` stroke, or an LED list, with a gapped pattern and a moving offset | 1 (lists), 4 (paths) |
| Noise field | `Paint2d::Noise`; brightness noise is a `Multiply` layer | 4 |
| Glitch | Dropout and hue shift filters, plus stutter on a nested effect | 4 |
| Heartbeat | Keys on an opacity track; speed-up is `Timeline::rate` from a parameter | 2 |
| Fill | A window whose length is a parameter (1d), or a `Rect` with `displace` for the moving surface (2d) | 1, 3 |
| Layers and masks | Layers, blend modes, masks, nested effects. "All but the roots" is an `LedQ` exclusion in frontbox-led | 3, 4 |
| Tempo sync | A track driven by the global beat parameter | 2 |

Other things RULES.md needs that shaped this plan: typed parameters for origins and accent colors, global parameters
for Steward integrity, timeline outros for effects that withdraw, 1d layers for hex rings, sparkle for "cracks flicker
across the playfield", and many instances of one effect at once (a packet per pop hit).

## Phases

Each phase lands with tests and leaves the old modules untouched. The order follows lotko's needs: radial wave, growth,
path flow, and layers and masks first.

### 0. Foundation

- [x] Add `serde`, `glam`, and `ron` dependencies
- [ ] `format` module: hex color serde (`#rrggbb` / `#rrggbbaa`), RON load/save, `version` checking
- [ ] `Sample` and `Span`

### 1. 1d

- [ ] `Paint1d`, `Fill1d`, `Window1d` with `sample_1d`
- [ ] `Layer1d`, `Scene1d`, and `Blend` (normal and add to start)
- [ ] Parity tests against `ColorSequence::generate` for the same descriptions
- [ ] RON round-trip tests: build in code, write, read back, same samples

### 2. Motion

- [ ] `Value`, parameter declarations, and instance and global scopes
- [ ] `Prop<T>`, `Track<T>`, `Key<T>`, `Driver`, reusing `Curve`, `Cycle`, and `Lerp`
- [ ] `Timeline` with intro/loop/outro segments and `rate`
- [ ] `Instance`: holds playback state and parameter values, `accumulate(delta)`, `play`/`stop`, completion
- [ ] Resolve an effect at the current time into a plain frame (no `Prop`s) that's cheap to sample many times
- [ ] Parity tests against `Tween` for equivalent tracks

### 3. 2d core

- [ ] `Shape` signed distance functions (circle, ring, rect, polygon), with coverage from `Sample::radius` and
      `softness`
- [ ] `Paint2d` solid, linear, radial, conic; `Transform`; `displace`
- [ ] `Layer2d`, `Scene2d`, the remaining blend modes, and masks by shape
- [ ] `Viewport` stage-to-device mapping
- [ ] Sampling tests at sparse points (playfield-like) and on a dense grid (DMD-like)

Radial waves, rect sweeps, and fills work at the end of this phase.

### 4. Composition, paths, growth, noise

- [ ] `Library` and nested effects, with time offset, rate, and parameters; masks by another layer's alpha
- [ ] Filters: dropout, hue shift, brightness, posterize
- [ ] `Path` shapes and strokes with a `Paint1d` along them
- [ ] `Shape::Growth`: seeded branch generation from origin toward target, cached geometry, `progress` reveal, trail
- [ ] `Paint2d::Noise` and `Paint2d::Sparkle`

### 5. Adapters (in frontbox-led and frontbox-dmd, alongside existing types)

- [ ] frontbox-led 1d: an `LedQ` and an effect instance → declarations
- [ ] frontbox-led 2d: a `ReferencePlane`, an `LedQ`, and an effect instance → project LED locations to stage points,
      sample with each LED's footprint → declarations
- [ ] frontbox-led fire-and-forget player: many independent instances of an effect, each removed when it completes,
      without a system having to own them
- [ ] frontbox-dmd: a pixel grid and pitch → sample each pixel → frame
- [ ] LED footprint: an optional size on `LedDefinition`, with a default per plane (frontbox-core)
- [ ] An owner for global parameters (beat position, game-wide values) that systems can set

### 6. Images

- [ ] `Image` paint: static images and gifs, by asset path, with the frame driven by a `Prop`

### 7. Migration

- [ ] Move `LedProgram1d` users and the DMD canvas onto the new types
- [ ] Delete `color_sequence`, `canvas`, and the trait-object parts of `animation`

### 8. Editor (later, mostly outside gfx)

- [ ] Hot reload of effect files on change, without recompiling
- [ ] Console editor: edits RON, previews live against real LED positions and a virtual DMD

## Open questions

- **Modulators.** `Modulator`/`MultiModulator` adjust a value on top of its base. Does that become a `Prop` variant
  (`Modulated { base, track, op }`) or go away in favor of tracks?
- **Time units in keys.** Seconds as `f32` read best in RON. Frame-relative timing is also on the TODO list.
- **Where footprint lives.** Per `LedDefinition`, per plane, or both with a fallback.
- **Nested effects: by name only, or inline too?** By name keeps files small and reusable; inline is handier for
  one-off pieces.
- **Growth tuning.** Which knobs (density, spread, jitter, branch angle, branch length) are worth exposing, and what
  the defaults are. Settle this by trying it on lotko's playfield.

## Later, outside gfx

- **Single-channel devices.** GI lamps and flashers have positions and should take part in effects. An adapter samples
  them like LEDs and drives them with the sample's brightness.
- **Beat position from frontbox-sound**, for tempo sync through the global beat parameter.
- **DMD UI.** frontbox-dmd already renders pixel fonts. Menus, lists, and text become a UI layer in frontbox-dmd that
  uses gfx for frames, boxes, outlines, and effects; text stays out of gfx for now.
- **3d.** True plane sweeps from the playfield into the backbox, and waves and noise that carry through the backbox
  speaker LEDs. Sampling per point makes this an addition later (`Vec3` samples; sphere, shell, half-space), not a
  redesign.
