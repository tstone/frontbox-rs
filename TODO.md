Before Show

- Add ability to re-order, shuffle, take, etc. ColorSequence?
- Wrap Vec<ExpansionBoard> in an ExpNetwork struct; make plural consistent IoBoards vs ExpBoard. Check why there is an IoBoard and IoBoards
- Defining an LED grid like strip, but with rows/cols and serpentine directions
- New frontbox-canvas crate that uses locations for 2d plane rendering via projections
- Operator configs should have some kind of "on config value changed" event
- Operator config changes should update HardwareValues automatically
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.

Nice to Have

- Streamline curve choices
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
