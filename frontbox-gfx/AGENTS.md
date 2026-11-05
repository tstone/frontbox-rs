# frontbox-gfx Agent Guidelines

These add to the repository's [AGENTS.md](../AGENTS.md). The roadmap is in [PLAN.md](PLAN.md).

## Design

- Prefer fewer, more flexible abstractions that combine to express many things over a large library of ready-made
  options. Never have two abstractions for one concept.
- Simplicity means fewer concepts, not shorter files. A wordier definition is fine if it uses fewer ideas; most scenes
  are written by the editor, templates, or agents, but they must stay readable.
- Common, high-traffic patterns can get syntactic sugar on top, once they're known to be common.
- Something that changes a shape or paint is a `Warp` (changes the position going in) or an adjust (changes the output
  coming out), added as an item in the node's list. Don't add it as a new field or a wrapper variant.

## No frontbox dependencies

frontbox-gfx depends on no other frontbox crate. It knows nothing about systems, hardware, LEDs, planes, or DMDs. It
may depend on general crates (`image`, `palette`, `glam`, `serde`, `ron`).

## Hardware binding stays out

gfx describes what a scene looks like and how it moves. Which LEDs it lands on (`LedQ`), which plane, z-index, and DMD
pixel grids all belong to the adapter crates (`frontbox-led`, `frontbox-dmd`). A scene file
never names hardware, so the same scene can go to a list of LEDs, the playfield, or a DMD.

## Definitions are data

gfx owns the definition of scenes and their storage format (RON).

- Definitions are enums and structs that derive `Serialize`/`Deserialize`. No trait objects, closures, or `fn`
  pointers in a definition. When there's a choice of kinds, it's an enum variant, not a trait implementation.
- Definitions hold no playback state. Elapsed time, cycle counts, and active flags live in the playback instance, never
  in the definition.
- A value that can change over time is a `Prop<T>`, not a bare `T`.
- Every stored file carries a `version`. Changing the format means bumping it.

## Sample, don't fill

gfx outputs things that can be sampled: a color for a position on the stage and the footprint around it. It never
allocates a buffer sized for a device. Devices decide where to sample.

## One model

There is no separate 1d model. A list of LEDs is laid out as a line on the stage, so every layer, shape, paint, blend,
mask, and filter works on strips, the playfield, and DMDs alike. Don't add a feature that only works on one kind of
target; if a strip needs something, build it so it also works on a plane.

## Units and color

- Coordinates are stage units, as `f32` (`glam::Vec2`). Not pixels. A sized stage is in real units; an unsized one
  takes the target's size (one unit per LED on a line).
- Colors are `Rgba<u8>`, with alpha, since LEDs and DMDs both composite. Definitions hold them as `Color`, a newtype
  that serializes as `(r, g, b, a)`, because `Rgba` has no serde support.

## Building alongside the old modules

`color_sequence`, `canvas`, and the trait-object parts of `animation` are being replaced. New work goes in new modules
next to them. Don't reshape the old modules to fit the new design; additive changes (like a `serde` derive) are fine.
The old modules are deleted in one migration once the new ones cover them.

## Tests

Every serializable type gets a RON round-trip test: build it in code, write it, read it back, and check that it samples
the same. New types that replace old ones get parity tests against the old behavior.
