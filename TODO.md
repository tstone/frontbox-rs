## Foundations

- Operator config, in order:
  1. One live instance. `App` loads operator config from disk and moves it into the read-only `BootSnapshot`
     (`ctx.config`, read by systems and driver setup), but the menu looks it up as a system with
     `ctx.expect::<ConfigCollection>()` and panics if none is running. Frontbox never starts one, so a
     game has to spawn its own `ConfigCollection::default()`: empty, not loaded from disk, and with no `app_sender`, so
     `ConfigChanged` is never emitted. The menu edits that copy and nothing else ever sees it. Frontbox should
     start the `App`'s instance as a system (like `MachineSystem`/`WatchdogSystem`) and have `ctx.config` and
     the menu methods both use it; games then drop their own `app.system(ConfigCollection::default())`. On its own this
     makes system-managed configs work, since they're read when used.
  2. Decide whether configs belong on the `System` trait. Today `fn configs()` on `System` is registered
     automatically, but only for startup systems added with `app.system()`; a system spawned at runtime has to be
     registered by hand with `app.register_configs()`, and if it isn't, its values silently read as defaults and never
     show up anywhere. That's easy to miss and a little too magical. Consider dropping it from `System` in favor of an
     explicit registration call (e.g. `app.configs(...)`), alongside the existing driver scan. Settle before key/label
     work, since it changes where registration happens. The metrics system's `fn metrics()` raises the same question.
  3. Separate the saved key from the menu label. `name` is both, so renaming a label loses the saved value and two
     values with the same name silently share an entry (`register` ignores duplicates). Panic on a duplicate key.
     Before saving lands, so keys on disk are stable.
  4. Save to disk. Keep the path (`load_from_disk` drops it), write on every change, keep `pending_disk` entries for
     values not registered yet, and if the file failed to parse, set it aside rather than overwrite it with defaults.
     Use `frontbox::toml`. Consider `toml_edit` so comments added by hand survive a save. Needs: step 1.
  5. Apply HardwareValue changes to the running machine. `MachineSystem::configure_driver` already re-resolves a mode
     against operator config and sends it, so the missing parts are: a key -> drivers index (can be built in
     `register_driver_configs`), and tracking the mode each driver currently has. On `ConfigChanged`,
     re-send the driver's *current* mode, not its definition's default: re-applying the default would undo a runtime
     change, e.g. enable flippers that are disabled while the operator is in the menu. Modes with no config values
     (disabled) resolve the same, so nothing is sent. Needs: step 1.
- Metrics system: generic, used by the framework (coil fires, switch hits, ball saves, eject failures) and exposed to
  user systems, declared statically like config values (registered the same way, see operator config step 2). Records
  over time rather than just sums, so values can be plotted and min/max/avg taken. Leaning towards one row per metric per game (counts kept
  in memory, written in one transaction at game end, plus a timer outside of games) rather than one per increment,
  which is thousands of rows a game. A reset is a marker, not a zeroing, so "since last reset" is a query and history
  is kept. Storage still open: needs crash-safe transactions and range scans, and ideally no C compilation burden for
  cross-compiling to the Pi. Candidates: redb (pure Rust key-value, ACID), Stoolap (pure Rust, SQL), sled (pure Rust,
  but long in beta), SQLite via rusqlite (`bundled` compiles C). With per-game rows, aggregating in Rust over a range
  scan is easy, so SQL isn't required. Needed by audits.
- Better identify which systems need interfaces (ball management, game management, rand system). Watchdog, LedSystem,
  Machine, etc. need to be configurable/swappable, which possibly means they need interfaces.
- So much is built on LedSystem that the framework should probably just start it up automatically. Needs: interfaces
  above, so it can still be swapped.
- Need to have a RandSystem that manages seed per game. Needed by event replay.
- System groups need to be able to contain other system groups (recursive parent lookup)
- Toggle group: a choice operator config where each choice names a system group (e.g. payment mode: free play vs credit
  play). A small provided system activates the chosen group at boot and swaps groups on `ConfigChanged` for
  that key, using the existing `activate_system_group`/`deactivate_system_group`. Choices need a saved key separate
  from the menu label, like config keys. Open question: should a swap wait until no game is in progress, for toggles
  that matter mid-game (difficulty, rules variants)? Needs: choice configs and operator config step 1.
- Hardware/Switch/LedDefinition should have a load from JSON functionality so that they can be edited via the console.
  Needed by the console LED designers.
- Support a virtual DMD that renders to the console, for developing and testing without hardware. Needed by the DMD
  menu animation and transition items.

## Pinball features

- Tilt: when tilted, ball save must stop saving before the balls drain (clearing it when the turn ends is too late, as
  the drains come first). Needs a tilt system, then e.g. a way to suspend ball save or have it listen for a tilt event
- Audits: the operator-facing view of metrics (coils fired, average game time, ball saves, etc.), with resets by
  category. Needs: metrics system.
- Ball management: a failed trough eject (`TroughEjectFailed`) currently drops the ball, which can leave a turn with no
  ball. Re-queue it with a growing backoff (recovers from power blips and jams) and emit a public event so the game can
  show it and record it in audits. Kick strength can be escalated with `MachineSystem::configure_driver`. Needs: audits,
  and tracking each driver's current mode (operator config step 5) so a config change doesn't undo the escalation. Open
  questions: retry forever or give up, and should `Launch::Auto` balls (e.g. multiball) be skipped sooner than
  `Launch::Player`
- Leaderboards: generalize high scores so the top 5 is one board among many (mode scores, fastest completions, etc.)
  that user systems can declare statically. A board has a name, direction (higher or lower is better), capacity (5, or
  1 for a champion), value kind (points, time, count) for display, and default entries. Mid-game achievements record a
  pending entry per player; at game end, initials entry runs once per player and the initials go on every board they
  made. That single end-of-game pass should also fix high scores being janky and needing too much system switching.
  Stored as TOML via `frontbox::toml`.
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
