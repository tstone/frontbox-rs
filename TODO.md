LEDs

- Implement binary versions of LED commands
- Single channel flasher support

LED Canvas

- Create a new frontbox-canvas crate
- combine DMD rendering + led canvas rendering
- standardize on image/Rgba<u8> for canvas

System

- Hardware selection by ID
- BUG: Cue::Once repeats
- Cue timeline
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.
- Auto plunger plugin has operator config for coil power/kick length
- System groups can contain groups
- Keyframe animation -- specify "this value, at this point in time" -- like tween but adjustable time durations between
- has_tag/has_typed_tag should probably be a trait

Hardware

- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

DX

- Some kind of console runner that shows switch states and has a terminal/console (this needs to skip the command listening part... somehow)
- Debugging: Some kind of websockets console to see what's going on (could this be an app plugin?)

Displays

- NeoSeg support
