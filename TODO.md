## Headless

- RandSystem (interface) that manages seed per game.
- Generalized systems: lane (single/multi-entrance), StartupEject (takes pairs of SwitchQ/DriverQ and fires drivers who have an active switch)
- ShotSystem which is a generalized system to handle emitting an event on shot hit (re-roll delay/dedupe, multi-entrance, optional hit scene)
- System::tagged as a "convention over configuration" alternative
- FreePlay becomes the "you passed this score threshold and earn a game"/knocker hit
- Config overrides that last for the duration of a game (e.g. tournament mode)
- Revisit if `Synchronize` event still makes sense or not

## Requires hardware/sensory QA

- Reference plane builder for standard pinball forms (Stern, Williams, Custom)
- Ball management: a failed trough eject (`TroughEjectFailed`) currently drops the ball, which can leave a turn with no ball. Re-queue it with a growing backoff (recovers from power blips and jams) and emit a public event so the game can show it and record it in audits. Kick strength can be escalated with MachineSystem::configure_driver`. Needs: audits. Open questions: retry forever or give up, and should `Launch::Auto`balls (e.g. multiball) be skipped sooner than`Launch::Player`
- Motor support: new MotorDefinition w/ FAST protocol + Machine support for motor commands
- Low-voltage drivers over the EXP bus, on a FAST FP-LUM-0709: one LED channel as a PWM output for things that aren't lights (motors, relays, etc.), owned by a system and kept out of light management so scenes can't turn them on. Core's boot check should count these as using a channel too, so a light and an output can't share one.
- Switch test mode for display menu
- Operator metrics can be reset (e.g. when a coil is replaced) lifetime vs current
- Config: No extra balls/points instead
- Metrics show on DMD menu

### Scenes/Editor
- Scene transitions (cut, cross-fade, wipe, push) between two playbacks. First version drives opacity or a mask on both; with nested scenes, a transition is a scene with two slots.
- Ability to compose LightQ in editor (are Reverse, Take, etc. still needed?)
- Changing a sound's volume while it plays (fades, modulators). Scene sound volume is a fixed number for now, as sounds are fire-and-forget; kira's `set_volume` on a kept handle could do it, but it may not belong in the scene.
- Proper BPM sync between music+scenes: a playback clock that follows music
- All the different scene features will probably need a library of demos/templates ("presets")

### Lighting

- Alternate resolver doesn't work with animated inputs: key its state on declaration ids, not colors
- Defining (hardware) an LED grid like strip, but with rows/cols and serpentine directions?
- Alphanumeric/NeoSeg support
- Evaluate if declaring named colors with FAST and reference by ID is worth it

### Sound

- frontbox-sound loop point support
- frontbox-sound multi-stem music support
- Schedule scene sound starts a tick ahead on the audio clock, for starts finer than a tick.
- apply-able effects (warble, reverb, glitch)
- legacy hardware emulation? built-in synthesis (frontbox-synth?) e.g. YM2610 FM

## DMD

- Implement remaining menu sounds
- Menu: editing a setting saves on every press, with no way back. Restore a setting's default from its edit screen, and repeat inc/dec while the button is held.
- Menu: fire a coil from its settings' edit screen
- Menu: decide what opening the coin door does to a game in progress, and whether the menu buttons do anything else
  during a game.
- Menu: ease left/right between sections (a scene transition, see Scenes/Editor)
- Anchoring or scaling so a single scene works on displays of different resolutions
- Menu: show the `MenuSection` tree in other places. The console's config view can list the whole tree at once, and an alphanumeric display needs its own system with its own navigation, as `DmdMenuSystem` has.
- Snap eased positions to whole pixels (`Transform::Snap(step)` or a display setting); the menu selection slide shimmers.
- Fit text: a text region with a list of fonts/sizes, picking the largest that fits.
- Brightness as an operator config, applied in the output.
- Global volume, SFX volume, music volume, callout volume all as a menu options
- Speed: skip a layer for samples outside its region's transformed bounds; build DMD graphs from the pixel grid instead of O(n²) `Graph::build`.
- Bundled framework scenes for sizes other than 128 x 32.
- DmdMatch system that takes a DMD scene or something. Its scene (`frontbox/128x32/match.scene`) reads `match`, and `player_1` to `player_4` (each score's last two digits); the numbers cycling before the match lands are keyframes in the scene.
- Ordered dithering in the `Pin2Dmd` output (6-bit channels band).
- Limited-palette `DisplayDevice` for classic mono DMDs (4 or 16 shades).

## LCD
- An LCD output: a new `DisplayDevice` and row-parallel sampling.

## Console

- Macros: recordable + re-playable switch/coil sequences (can also reference other macros)
- Event sourcing/replay - dump per game (serialize to JSON) - record timestamp as a well. Needs: RandSystem, so a replayed game is deterministic.

## Docs and tooling

- Have a Claude skill that's ready to go for people (how can this not duplicate docs?). Last, so it describes the API after the renames and cleanups above.
- There seems to be a layering emerging: setup hardware + verify that works; setup systems for hardware (Scoop) and shots (ShotSystem); build modes on top of that
