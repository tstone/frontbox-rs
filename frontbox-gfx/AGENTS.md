# frontbox-gfx Agent Guidelines

These add to the repository's [AGENTS.md](../AGENTS.md). The roadmap is in [PLAN.md](PLAN.md).

## No frontbox dependencies

frontbox-gfx depends on no other frontbox crate. It knows nothing about systems, hardware, LEDs, planes, or DMDs. It
may depend on general crates (`image`, `palette`, `glam`, `serde`, `ron`).

## Hardware binding stays out

gfx describes what an effect looks like and how it moves. Which LEDs it lands on (`LedQ`), which plane, z-index, and DMD
pixel grids all belong to the adapter crates (`frontbox-led`, `frontbox-dmd`). An effect file
never names hardware, so the same effect can go to a list of LEDs, the playfield, or a DMD.

## Definitions are data

gfx owns the definition of effects and their storage format (RON).

- Definitions are enums and structs that derive `Serialize`/`Deserialize`. No trait objects, closures, or `fn`
  pointers in a definition. When there's a choice of kinds, it's an enum variant, not a trait implementation.
- Definitions hold no playback state. Elapsed time, cycle counts, and active flags live in the playback instance, never
  in the definition.
- A value that can change over time is a `Prop<T>`, not a bare `T`.
- Every stored file carries a `version`. Changing the format means bumping it.

## Sample, don't fill

gfx outputs things that can be sampled: a color for a `t` along a line (1d), or for a position and footprint (2d). It
never allocates a buffer sized for a device. Devices decide where to sample.

## Units and color

- 2d coordinates are real units on a stage, as `f32` (`glam::Vec2`). Not pixels.
- Colors are `Rgba<u8>` in memory, with alpha, since LEDs and DMDs both composite. In files they're hex strings that
  keep alpha: `"#rrggbb"` (opaque) or `"#rrggbbaa"`.

## Building alongside the old modules

`color_sequence`, `canvas`, and the trait-object parts of `animation` are being replaced. New work goes in new modules
next to them. Don't reshape the old modules to fit the new design; additive changes (like a `serde` derive) are fine.
The old modules are deleted in one migration once the new ones cover them.

## Tests

Every serializable type gets a RON round-trip test: build it in code, write it, read it back, and check that it samples
the same. New types that replace old ones get parity tests against the old behavior.
