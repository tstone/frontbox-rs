# frontbox-gfx Agent Guidelines

These add to the repository's [AGENTS.md](../AGENTS.md).

## Design

- Prefer fewer, more flexible abstractions that combine to express many things over a large library of ready-made
  options. Never have two abstractions for one concept.
- Simplicity means fewer concepts, not shorter files. A wordier definition is fine if it uses fewer ideas; most scenes
  are written by the editor, templates, or agents, but they must stay readable.
- Common, high-traffic patterns can get syntactic sugar on top, once they're known to be common.
- Something that changes a shape or fill is a `Transform` (changes the position going in) or an adjust (changes the output
  coming out), added as an item in the node's list. Don't add it as a new field or a wrapper variant.

## No frontbox dependencies

frontbox-gfx depends on no other frontbox crate. It knows nothing about systems, hardware, LEDs, planes, or DMDs. It
may depend on general crates (`image`, `palette`, `glam`, `serde`, `ron`).

## Hardware stays out of this crate

gfx describes what a composition looks like and how it moves. Which LEDs it lands on (`LedQ`), which plane, z-index, and
DMD pixel grids all belong to the adapter crates (`frontbox-led`, `frontbox-dmd`). Each has its own scene type, a
`GfxComposition` plus what that hardware needs, and that's what's written to a `.scene` file. So a scene file does name
hardware, and an LED scene and a DMD scene aren't interchangeable. Only this crate stays free of it.

## Definitions are data

gfx owns the definition of compositions and the storage format of scene files (RON).

- Definitions are enums and structs that derive `Serialize`/`Deserialize`. No trait objects, closures, or `fn`
  pointers in a definition. When there's a choice of kinds, it's an enum variant, not a trait implementation.
- Definitions hold no playback state. Elapsed time, cycle counts, and active flags live in the playback instance, never
  in the definition.
- A value that can change over time is a `Prop<T>`, not a bare `T`.
- Every stored file carries a `version`. Changing the format means bumping it, for as long as there are files to keep
  reading. While everything is on this branch, files are simply rewritten.

## Sample, don't fill

gfx outputs things that can be sampled: a color for a position in the scene and the footprint around it. It never
allocates a buffer sized for a device. Devices decide where to sample.

## One model

There is no separate 1d model. A `Surface` is either a `Sequence` of points in order or an `Area`, and that's the only
difference: every layer, shape, fill, blend, mask, and filter works in both. Don't add a feature that only works in one;
if a strip needs something, build it so it also works on a plane.

A device maps its points into the viewport, so a composition never changes with a device's resolution.

## Units and color

- Coordinates are scene units, as `f32` (`glam::Vec2`). Not pixels. They mean whatever the viewport's size says: a
  scene for a DMD might use pixels, one for the playfield inches.
- Colors are `Rgba<u8>`, with alpha, since LEDs and DMDs both composite. Definitions hold them as `Color`, a newtype
  that serializes as `(r, g, b, a)`, because `Rgba` has no serde support.

## Building alongside the old modules

`canvas` and the trait-object parts of `animation` are being replaced. New work goes in new modules
next to them. Don't reshape the old modules to fit the new design; additive changes (like a `serde` derive) are fine.
The old modules are deleted in one migration once the new ones cover them.

## Tests

Every serializable type gets a RON round-trip test: build it in code, write it, read it back, and check that it samples
the same. New types that replace old ones get parity tests against the old behavior.
