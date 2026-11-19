# Plan: Lights

Today the LED system only knows RGB LEDs on the EXP bus. A start button lamp on a low-voltage driver lives outside
it, flashed by hand from `CreditsSystem` with its own timer. This plan makes "a light" the thing systems and scenes
address, whatever turns it on, so a lamp gets declarations, flashing, z-layers, deactivation with its system, and
scene placement the same way an LED does.

## The model

A **light** is a light source that turns on. It has a name, tags, a position, and an address, which says where it's
wired:

```rust
enum LightAddress {
  /// Written over the EXP bus as part of an LED address's RGB
  Exp { led: LedAddress, channels: Channels },
  /// A driver powering a bulb
  Driver { driver: &'static str },
}

enum Channels {
  /// All three, as one color (an RGB LED)
  Rgb,
  /// One of them, as one brightness
  One(Channel), // R, G or B
}
```

An **`LedAddress`** is one index on an EXP board's LED port, `(board, port, index)`, which FAST writes as one RGB
triplet. It keeps its name: EXP boards have other kinds of ports (motor ports, for one), and this is specifically a
place on an LED port. What's wired to it might be one RGB LED or up to three single-channel lights.

These are the only two kinds of address, because they're all FAST gives: RGB over the EXP bus, and drivers.

A light's address is also its identity. No two lights share one (for single-channel lights, core's boot check makes
sure of that), so light management and its callers name a light by its `LightAddress`, and there's no separate id.
Within a run an address never changes. Anything saved, such as scene files, names lights by name.

A light with `Channels::Rgb` shows a color. Every other light shows one brightness. Declarations and scenes keep
using colors everywhere, and a single-brightness light reduces its color to brightness as the largest channel after
compositing over black, so alpha dims it.

### Core describes the machine; lighting runs the lights

`frontbox-core` holds what's true about the machine, and works without `frontbox-lighting`:

- **Which lights exist**: name, tags, location, and where each is wired (`LightAddress`).
- **What a driver powers**: `DriverLoad::{Coil, Light}` on the driver.
- **The bulb's color** of a driver light, which the console uses for previews. A red lamp is red.
- **Light radius**: how far a light spreads on its plane (today's `led_radius` in `AppConfig`). This is a physical
  size, like location, and the console reads it from `AppConfig` through the tracer to preview scenes. It stays in
  core, renamed `light_radius`. If lights ever differ in size, it moves onto the light.
- **The lookup and query**: `LightLookup`, `LightQ` and planes. `frontbox-scene` uses `LightQ` without depending on
  the lighting crate.
- **Raw writes**: `ctx.lights.set(...)` for LED addresses, and `ctx.drivers` for driver lights.

`frontbox-lighting` holds how lights are run: declarations, composing, flashing, playing scene layers, and lowering a
light's value to its address. Its settings live with light management, not in core:

- **Threshold**: the brightness at or above which a driver light is on. 0.5 by default, with per-light overrides.

A game that doesn't use `frontbox-lighting` still gets the description of its lights, sees them in the console, and
can write them raw.

### Light management

Light management is the single owner of the lights, as ball management is of the balls. It's the only thing that
writes LED addresses and driver lights. Every other system, and every scene, declares through it.

It follows the same pattern as ball and game management:

- **`LightManager`**: the trait. Its methods are the public surface: declare, undeclare, activate and deactivate by
  system, and conflict resolution. Other systems look it up with `ctx.systems.get_dyn::<dyn LightManager>()`.
  Its methods say "this light shows this color", with plain arguments:

  ```rust
  fn declare(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], flashing: bool);
  fn declare_inactive(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], flashing: bool);
  fn undeclare(&mut self, owner: u64, z: i8, lights: &[LightAddress]);
  fn undeclare_by_system(&mut self, owner: u64);
  fn activate_by_system(&mut self, owner: u64);
  fn deactivate_by_system(&mut self, owner: u64);
  fn set_conflict_resolution(&mut self, lights: &[LightAddress], resolution: LightConflictResolution);
  fn declared_colors(&self, light: &LightAddress) -> Vec<Rgba<u8>>;
  ```

  z belongs to a declaration, not to a light or a query: a light has no z of its own, only the declarations on it do.
- **`LayeredLightManager`**: the implementation this crate provides, formerly `LedSystem`. It lists
  `dyn LightManager` in `System::provides`.
- **`LightManagerExt`**: keeps `declare_lights` and the rest working on `SystemContext`, resolving a `LightQ` to
  addresses, and adds `ctx.light_manager()`.

It spawns no children. Product support (see below) is made of peers that declare through it, never children of it.

### One frame of `LayeredLightManager`

1. **Gather** declarations from systems, and colors sampled from the scene layers it plays.
2. **Compose per light**: z-layers, conflict resolution, flashing.
3. **Lower to addresses**:
   - `Exp` lights write their channels into their LED address.
   - `Driver` lights compare brightness to their threshold and become on or off.
4. **Send only what changed**:
   - LED addresses go out as FAST's RGB commands.
   - Driver lights go out as virtual switch on or off.

### Kinds, not products

Light management knows lights and the two kinds of address, and nothing about any specific product. Support for
products is built on top:

- **Single-channel flashers**: single-channel lights on a shared LED address, made with a definition helper. They
  need no new code in light management.
- **NeoSeg**:
  - A definition helper turns a digit into its segment lights.
  - Text comes from a separate system holding the character table, which declares on those lights through the public
    API.

## Names

| Today | After | Why |
|---|---|---|
| `LED` (the runtime struct) | `Light` | Any light source, not only LEDs |
| `LedQ` | `LightQ` | |
| `LedLookup`, `ctx.leds` | `LightLookup`, `ctx.lights` | |
| `LedSystem` | `LightManager` (trait), `LayeredLightManager` (implementation) | The trait follows `BallManager`. The implementation is named for how it works, composing in z-layers, as other approaches are possible |
| `LedSystemExt` | `LightManagerExt` | |
| `declare_leds`, `declare_leds_flashing`, `declare_led_colors`, `declare_leds_inactive`, `undeclare_leds`, `activate_led_declarations`, `deactivate_led_declarations`, `set_led_conflict_resolution` | `declare_lights`, ... (`led` → `light` throughout) | |
| `LedConflictResolution` | `LightConflictResolution` | |
| `LedDeclarations`, `LedIdentifications`, `LedIdentificationsExt` (`.at_z`), `IdentificationAtZ` | removed (phase 2) | The trait takes plain arguments, and z is passed with the declaration |
| `LayeredLightManager::monitor` | removed (phase 2) | Nothing calls it |
| `LedPlacement`, `LedLayer`, `LedSurface`, `SceneLayer::Leds` | `LightPlacement`, `LightLayer`, `LightSurface`, `SceneLayer::Lights` | |
| `led_radius`, `DEFAULT_LED_RADIUS`, `set_led_radius` | `light_radius`, ... (stays in core) | A physical size, which the console needs too |
| crate `frontbox-led` | `frontbox-lighting` | The crate is the subject, lighting; what it manages are lights |
| log target `frontbox::led` | `frontbox::lighting` | Update `doc/logging.md` |
| `LedAddress` (as a light's identity) | `LightAddress` (phase 2) | A light's address, of which `LedAddress` is the EXP case |
| `LedAddress` (as a place on an LED port) | unchanged | It's specifically an LED port, among the kinds of EXP port |
| `LedDefinition` | unchanged | It defines LEDs, which are one kind of light |

The thing is a "light" and not a "lighting": "lighting" suggests making a space brighter, where a light here is a
source that turns on. "Lighting" names only the subject: the crate and the log target.

## Phases

Each phase builds, passes tests, and leaves the machine behaving as before, except where a phase says otherwise.

### 2. Address lights by `LightAddress`

Behavior doesn't change. The only address is `Exp { led, channels: Rgb }`.

- **The address.** Add `LightAddress` and `Channels` to core. `LED` gets `address: LightAddress` in place of its
  `LedAddress`, which moves inside it.
- **The trait.** `LightManager` takes the plain arguments shown under [Light management](#light-management): lights
  by `LightAddress`, and z passed with each declaration.
  - Remove `LedDeclarations`, `LedIdentifications`, `LedIdentificationsExt` and `IdentificationAtZ`. Queries resolve
    to lights only.
  - The ext's methods take z as an argument where today's callers use `.at_z(...)` (16 here, 14 in lotko).
  - Systems that store a target as `Box<dyn Contextual<LedIdentifications>>` store a `LedQ` instead, plus a z field
    of their own if they need a fixed one.
  - `set_conflict_resolution` takes lights only. It takes a z today and ignores it.
- **Remove `monitor`.** Nothing calls it.
- **Inside `LayeredLightManager`.**
  - Declarations, `prior_render`, `played`, the alternate resolver and `LedPlacement` are keyed by `LightAddress`.
  - Lowering groups lights by `LedAddress`.
  - The public `led_color()` becomes the private lowering step.
- **Console and trace.** LED state is keyed by `LightAddress`.

### 3. Rename

Everything else in the [Names](#names) table, in one change.

- **Scene files.** `SceneLayer::Leds` and `LedLayer.leds` are written into scene files. They're renamed outright
  (`Lights`, `lights`), with no aliases, and old files no longer load. This repo's scenes have no LED layers. Lotko's
  scenes are migrated in the same change (`Leds(` → `Lights(`, `leds:` → `lights:` inside those layers).
- **The console.** Regenerate the TS types (`ts` feature). Then update `frontbox-console/web`: `sceneModel.ts`,
  `LedPicker.tsx` (→ `LightPicker.tsx`), `configNodes.ts`, `LayerPlace.tsx`.
- **Log target.** `frontbox::led` → `frontbox::lighting`, and `doc/logging.md`.
- **Docs.** README, doc comments and examples.

### 4. Driver lights

- **What a driver powers.** Add `DriverLoad::{Coil, Light}` to `DriverDefinition` and the runtime `Driver`.
  - `DriverDefinition::new` sets `Coil`.
  - `DriverQ` can select on it (`DriverQ::load(DriverLoad::Coil)`, with `DriverQ::coils()` as shorthand).
  - Ball search fires only coils, whatever query it's given. A lamp tagged `Playfield` is never pulsed.
- **Definition.** `DriverDefinition::light(name)` sets `Light` and takes `.color(Rgba)` (the bulb's color),
  `.location(Vec3)` and `.tag(...)`. It configures the driver as today (`pulse_hold`, virtual switch, 0% initial,
  100% hold) and registers a light with a `Driver` address.
- **The lookup.** `LightLookup` takes lights from the IO network as well as EXP boards.
- **Threshold.** A `LayeredLightManager` setting, 0.5 by default, with per-light overrides by `LightQ`.
- **Writing.** `LayeredLightManager` sends `activate(VirtualSwitchOn)` / `deactivate(VirtualSwitchOff)` on a
  change, and forgets a driver light's last state on `reset()` as it does for LED addresses.
- **`CreditsSystem`.**
  - `start_lamp` takes a `LightQ`.
  - It declares the lamp flashing while a press would add a player, and takes the declaration back otherwise.
  - The `LampOn`/`LampOff` timer, `lamp()` and `start_lamp_driver` go away. So does the forced `.tag(Cabinet)`.
  - `tests/credits.rs` defines the lamp with `DriverDefinition::light`.
- **Delete** `LampDefinitionBuilder` and `_FrontboxDrivenLamp`.
- **`TODO.md`.** Remove "Implement a proper LampDriver". In `REPORT-4-PINBALL.md` (B7), the lamp half is done.

### 5. Single-channel lights on an LED address

- **Definition.** `LedDefinition::single` takes a channel (`.channel(Channel::G)`), giving `Channels::One`.
- **A generic helper for 1 to 3 lights on one LED address**, in R, G, B order.
- **Core's boot check** is only that no channel is used by two lights, including an `Rgb` light, which uses all three.
  Reject the config with an error naming the LED address and both lights. This is what makes a `LightAddress`
  unique to one light.
- **The flasher helper** defines a chain of single-channel flashers on a port. The boards take channels one after
  another along the chain, so the helper assigns LED addresses and channels in that order. Only the last LED address
  can be partly used. The rule lives in the helper because it's how flasher boards read the chain, not a rule about
  LED addresses. Other
  hardware, such as a NeoSeg digit, can leave channels unused mid-port.
- **`TODO.md`.** Remove "Single channel flasher support".

### Later, not in this plan

- **NeoSeg** as its own module. Nothing about it is added to `LightAddress` or `Channels`.
  - **Segments.** Each segment is a single-channel light (`Exp { led, channels: One(..) }`) with a name such as
    `score.3.g` and a position from the segment layout. Scenes see only these lights, so sweeps across the display
    work with nothing NeoSeg-specific.
  - **The definition helper.** It takes a port, a starting index, a digit count and a layout, and produces the
    segment lights and a `SegmentDisplay`: per digit, the light for each segment, in the character table's order.
  - **Channel order.** Brief testing showed the channels out of the expected order, possibly because segments pack
    across LED address boundaries, so one digit starts partway through the previous one's last LED address. The helper
    owns working out LED addresses and channels, so either layout fits. Work out the real order on hardware first.
  - **The text system.** It takes the `SegmentDisplay`, looks each character up in its table, and declares brightness
    on the segment lights through light management.
- **Text from scenes.** Playing text from a scene needs a new layer kind, and `SceneLayer` is a closed enum in
  `frontbox-scene`. Decide then whether that kind is generic ("text on a named character display") or whether scenes
  get a way to add layer kinds from outside.
- **Pulsed driver lights**, such as incandescent flashers on high-voltage drivers. `DriverLoad::Light` already keeps
  them out of ball search. Lowering one to a pulse instead of a hold is a second way to write a `Driver` address.
- **PWM dimming** of driver lights (by reconfiguring hold power).
- **Hysteresis** on the threshold, if a scene shows a lamp chattering.

## Check afterwards

- **Performance of keying by `LightAddress`.** Light management keys its per-frame work by `LightAddress`, an enum,
  rather than by an index. A real machine has about 250 to 350 LEDs. DMD-sized displays don't go through light
  management, and what usually pushes the count higher is cabinet lighting, such as a 3m strip. Once phases 2 to 5
  are in, profile a frame at that size and at a few long strips' worth more. If hashing addresses shows up, give
  lights an index inside `LayeredLightManager`, without changing the trait.
