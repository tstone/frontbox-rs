## Foundations

- Some kind of persistable storage (re-use Store, but add Deserialize requirement). Needed by audits, high scores and
  operator config.
- Better identify which systems need interfaces (ball management, game management, rand system). Watchdog, LedSystem,
  Machine, etc. need to be configurable/swappable, which possibly means they need interfaces.
- So much is built on LedSystem that the framework should probably just start it up automatically. Needs: interfaces
  above, so it can still be swapped.
- Need to have a RandSystem that manages seed per game. Needed by event replay.
- Runtime driver config changes. Needed by trough eject retry (escalating kick strength).
- Operator config changes should update HardwareValues automatically -- is this a system that listens to config change
  events? Needs: persistable storage.
- System groups need to be able to contain other system groups (recursive parent lookup)
- Maybe some kind of "toggle" group that can rotate between one of N (e.g. free play vs credit play). Needs: recursive
  system groups.
- Hardware/Switch/LedDefinition should have a load from JSON functionality so that they can be edited via the console.
  Needed by the console LED designers.

## Pinball features

- Tilt: when tilted, ball save must stop saving before the balls drain (clearing it when the turn ends is too late, as
  the drains come first). Needs a tilt system, then e.g. a way to suspend ball save or have it listen for a tilt event
- Audits: Keep stats on coils fired, etc. Needs: persistable storage.
- Ball management: a failed trough eject (`TroughEjectFailed`) currently drops the ball, which can leave a turn with no
  ball. Re-queue it with a growing backoff (recovers from power blips and jams) and emit a public event so the game can
  show it and record it in audits. Needs: audits, runtime driver config changes (to escalate the kick strength). Open
  questions: retry forever or give up, and should `Launch::Auto` balls (e.g. multiball) be skipped sooner than
  `Launch::Player`
- High scores are janky and require too much system switching (but how else to support multiple input methods?).
  Persisting them needs: persistable storage.
- Stability: Robust handling for USB disconnects/reconnects
- Shutdown should probably have some way of configuration an FnOnce instead of the current system that hard-codes
  linux-specific shutdown handling

## API cleanup

- ExpAddress is defined twice, once in fast-protocol and once in frontbox. The latter has "port". This needs to be
  resolved.
- Clean up hardware exports
- With fb-pin centralized, maybe events should be centralized too
- Establish (and document) consistent log targets

## Motors

- New MotorDefinition
- FAST protocol
- Update Machine to support motor commands

## LEDs and animation

- Really should revert name back to "effect" which is so much more natural LedEffect1d, LedEffect2d
- Should "timeline" be renamed "keyframe"?
- LedQ::any naming is weird, because it's basically saying "all of these" but it's written like a query "any of these
  are true"
- While LedProgram makes things feel normal, it's still a little weird how sometimes things are still manually declared.
  Feels like there needs to be a unified API.
- Because of the way LedProgram1d is "owned" by the system, the system has to hang around to let the effect complete.
  This makes system replacement unnecessarily difficult, and creates weird "lag" feeling in the software. Instead there
  could perhaps be a "LedProgramOneShot" system which works more play `play_sfx` where something like `play_effect`
  could be called it places through, then removes itself independent of the system starting it. This would need to be
  ServiceContext to keep it alive and that may result in overlapping declarations (perhaps LedSystem itself manages
  these and doesn't auto-remove on despawn?).
- Animations should be able to be specified as relative to the frame rate
- Single channel flasher support
- Defining an LED grid like strip, but with rows/cols and serpentine directions -- ColorMatrix instead of ColorSequence?
- Expand named color library
- Maybe: Declare library colors with FAST and reference by ID
- NeoSeg support
- LedSystem should maybe break away to be it's own crate? maybe animation too, and implement palette for HSL/color
  modifications

## Sound

- frontbox-sound loop point support
- frontbox-sound multi-stem music support
- legacy hardware emulation? e.g. YM2610 FM

## DMD

- DMD package probably needs to become frontbox-dmd since most of the code could support other DMDs
- Implement all menu sounds
- Transition left/right ease between sections
- Animate right offset of section arrow when selected
- Fancy: the selection box animates between vertical offsets
- Support a virtual DMD that renders to the console (maybe?)

## Canvas

- Modulations should work with canvas
- Uses locations for 2d plane rendering via projections
- Reference plane stitching
- Fill2d needs a perlin noise fill
- combine DMD rendering + led canvas rendering. Needs: frontbox-dmd split.

## Console

- Initial state: configure switches to be latched by default (e.g. trough, captive ball)
- Macros: recordable + re-playable switch/coil sequences (can also reference other macros)
- Event sourcing/replay - dump per game (serialize to JSON) - record timestamp as a well. Needs: RandSystem, so a
  replayed game is deterministic.
- LedEffect1d web based designer: uses an API to render actual colors. Needs: load from JSON, LED renames.
- LedEffect2d web based designer: same as above, but also uses LED config

## Docs and tooling

- Have a Claude skill that's ready to go for people (how can this not duplicate docs?). Last, so it describes the API
  after the renames and cleanups above.
