Before Show

- Add ability to re-order, shuffle, take, etc. Contextual<LedIdentifications> (or should this be a mutation of ColorSequence?)
- Defining an LED grid like strip, but with rows/cols and serpentine directions
- New frontbox-canvas crate that uses locations for 2d plane rendering via projections
- Auto plunger plugin has operator config for coil power/kick length
- OperatorConfig should be a separate System -- this will need a way for plugins to register data for systems that aren't yet started
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.

Nice to Have

- combine DMD rendering + led canvas rendering
- LedSystem should maybe break away to be it's own crate? maybe animation too, and implement palette for HSL/color modifications
- Single channel flasher support
- NeoSeg support
- System groups can contain groups
- frontbox-sound multi-stem music support
- frontbox-sound loop point support

LEDs

- Implement binary versions of LED commands
- Keyframe animation -- specify "this value, at this point in time" -- like tween but adjustable time durations between
- has_tag/has_typed_tag should probably be a trait
- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

DX

- Figure out how to do step debugging on a live machine
