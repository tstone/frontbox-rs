LEDs

- Introduce idea of "scopes" e.g. `let leds = systems.expect_mut::<LedSystem>().scope(ctx.current_system_id())`
- Implement binary versions of LED commands
- Update docs
- Single channel flasher support

System

- Get neuron version and check >= rev 6 to support ER
- BUG: Cue::Once repeats
- Cue timeline
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.
- Auto plunger plugin has operator config for coil power/kick length
- System groups can contain groups
- Keyframe animation -- specify "this value, at this point in time" -- like tween but adjustable time durations between
- Should events be required to be Serialize?
- has_tag/has_typed_tag should probably be a trait

Hardware

- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

DX

- Some kind of console runner that shows switch states and has a terminal/console (this needs to skip the command listening part... somehow)
- Debugging: Some kind of websockets console to see what's going on (could this be an app plugin?)

Displays (as a System)

- FAST LED canvas
- NeoSeg support
