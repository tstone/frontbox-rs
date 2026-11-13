## Headless

- Establish (and document) consistent log targets
- Reference planes should allow polygons so that cabinet sides don't look weird
- RandSystem (interface) that manages seed per game.
- LedSystem becomes the implementation of an interface
- Shutdown should probably have some way of configuration an FnOnce instead of the current system that hard-codes linux-specific shutdown handling
- ExpAddress is defined twice, once in fast-protocol and once in frontbox. The latter has "port". This needs to be
  resolved.
- Generalized systems: Scoop, lane (single/multi-entrance), StartupEject (takes pairs of SwitchQ/DriverQ and fires drivers who have an active switch)
- CrashPowerDown -- Verify that this still makes sense
- Operator metrics can be reset (e.g. when a coil is replaced)
- Config: No extra balls/points instead
- With fb-pin centralized, maybe events should be centralized too
- System groups need to be able to contain other system groups (recursive parent lookup)
- Start pressed while a game can't be started (balls from the last game still coming home, a game ending) is ignored without a word. Emit an event for it (e.g. `GameStartRefused`, with the reason) so a game can attach feedback: a sound, a DMD flash, a lamp.
- Toggle group: a choice operator config where each choice names a system group (e.g. payment mode: free play vs credit play). A small provided system activates the chosen group at boot and swaps groups on `ConfigChanged` for that key, using the existing `ctx.groups.activate`/`ctx.groups.deactivate`. Choices need a saved key separate from the menu label, like config keys. Open question: should a swap wait until no game is in progress, for toggles that matter mid-game (difficulty, rules variants)?
- Some kind of bundle that makes all the pinball things start up

## Design Choices/Core

- Implement a proper LampDriver that also supports flashing
- Revisit if `Synchronize` event still makes sense or not
- Should a "hold" (like how BallSave uses them) be a framework feature?
- Clean up hardware exports

## Requires hardware/sensory QA

- Leaderboards: generalize high scores so the top 5 is one board among many (mode scores, fastest completions, etc.) that user systems can declare statically. A board has a name, direction (higher or lower is better), capacity (5, or 1 for a champion), value kind (points, time, count) for display, and default entries. Mid-game achievements record a pending entry per player; at game end, initials entry runs once per player and the initials go on every board they made. That single end-of-game pass should also fix high scores being janky and needing too much system switching. Stored as TOML via `frontbox_core::toml`.
- Reference plane builder for standard pinball forms (Stern, Williams, Custom)
- Ball management: a failed trough eject (`TroughEjectFailed`) currently drops the ball, which can leave a turn with no ball. Re-queue it with a growing backoff (recovers from power blips and jams) and emit a public event so the game can show it and record it in audits. Kick strength can be escalated with MachineSystem::configure_driver`. Needs: audits. Open questions: retry forever or give up, and should `Launch::Auto`balls (e.g. multiball) be skipped sooner than`Launch::Player`
- Motos support: new MotorDefinition w/ FAST protocol + Machine support for motor commands

### Scenes/Editor

- Line-based fills (polyline, spiral, logarithmic spiral, etc. snake patterns) 
- Line based fill patterns usable for Sweep
- Graph algorithms have a "Reverse" option to perform the algo in reverse (ie. calculate then play backwards)
- Noise and noise masks (some kind of transparency threshold adjust)
- Need to probably move pixel font rendering into gfx with real TTF rendering
- New curves for flame/flicker, stutter, glitch
- Ability to compose curves in editor
- Ability to compose LedQ in editor (are Reverse, Take, etc. still needed?)
- Plane stitching for playback (or should this just be real 3d rendering?)
- SVG import for layer shapes
- Animations should be able to be specified as relative to the frame rate
- Proper BPM sync between music+scenes

### LEDs

- Alternate resolver doesn't work with animated inputs
- FirstWins isn't first. Hashmap does not guarantee order
- LedSystem becomes an interface
- Because of the way LedScene is "owned" by the system, the system has to hang around to let the effect complete. This makes system replacement unnecessarily difficult, and creates weird "lag" feeling in the software. Instead there could perhaps be a "LedProgramOneShot" system which works more play `play_sfx` where something like `play_effect` could be called it places through, then removes itself independent of the system starting it. This would need to be ServiceContext to keep it alive and that may result in overlapping declarations (perhaps LedSystem itself manages these and doesn't auto-remove on despawn?).
- Defining (hardware) an LED grid like strip, but with rows/cols and serpentine directions?
- Single channel flasher support
- NeoSeg support
- Evaluate if declaring named colors with FAST and reference by ID is worth itc

### Sound

- Some kind of reconnect with backoff to the sound service/driver
- frontbox-sound loop point support
- frontbox-sound multi-stem music support
- music track repeat
- apply-able effects (warble, reverb, glitch)
- legacy hardware emulation? built-in synthesis (frontbox-synth?) e.g. YM2610 FM

## DMD

- DmdSystem becomes an interface
- DMD package probably needs to become frontbox-dmd since most of the code could support other DMDs
- Retire `frontbox_gfx::canvas` (and `extent`) by moving DMD rendering onto scenes. Needs: DMD scene playback, text in scenes (pixel and sprite-sheet fonts), and a way to lay out the menu. Its `Size`/`Position` are pixel sizes, so the DMD keeps an integer size of its own.
- DmdSystem should listen for MachineDisconnected/Reconnected events and automatically show something on the DMD if that happens (z-index 100?)
- Implement remaining menu sounds
- Need a better pattern/more generalized UI DSP to do the following:
- Transition left/right ease between sections
- Animate right offset of section arrow when selected
- Fancy: the selection box animates between vertical offsets
- Support a virtual DMD that renders to the preview console, for developing and testing without hardware. Needed by the DMD menu animation and transition items.

## Console

- Initial state: configure switches to be latched by default (e.g. trough, captive ball)
- Some kind of game state specific/user-customizable JSON emission by systems, to view a system's state somehow
- Macros: recordable + re-playable switch/coil sequences (can also reference other macros)
- Event sourcing/replay - dump per game (serialize to JSON) - record timestamp as a well. Needs: RandSystem, so a
  replayed game is deterministic.

## Docs and tooling

- Have a Claude skill that's ready to go for people (how can this not duplicate docs?). Last, so it describes the API
  after the renames and cleanups above.
