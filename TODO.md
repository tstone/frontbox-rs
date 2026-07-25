- There needs to be some kind of generic "machine state" system -- maybe machine state is a tag -- that everything else can react to
  States: attract, door open, in-game

- Context extensions need to NOT work unless they are imported
- Wrap Vec<ExpansionBoard> in an ExpNetwork struct; make plural consistent IoBoards vs ExpBoard. Check why there is an IoBoard and IoBoards
- Operator config changes should update HardwareValues automatically -- is this a system that listens to config change events?
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.

Canvas

- Modulations should work with canvas
- Uses locations for 2d plane rendering via projections
- Fill2d needs a perlin noise fill
- Reference plane stitching

DMD Menu

- How will DMD menu system know about the sound system (interface?)
- Menu up/down navigation doesn't work
- Need to be able to edit a config
- Animate right offset of section arrow when selected
- Transition left/right ease between sections
- Fancy: the selection box animates between vertical offsets

Nice to Have

- Defining an LED grid like strip, but with rows/cols and serpentine directions -- ColorMatrix instead of ColorSequence?
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
