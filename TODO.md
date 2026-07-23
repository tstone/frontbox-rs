- Context extensions need to NOT work unless they are imported
- Wrap Vec<ExpansionBoard> in an ExpNetwork struct; make plural consistent IoBoards vs ExpBoard. Check why there is an IoBoard and IoBoards
- Operator config changes should update HardwareValues automatically -- is this a system that listens to config change events?
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.

Canvas

- Render whole image + sprite sheet off disk
- Remove duplicate elements from DMD (keep pixel font rendering in DMD)
- Pixel font rendering should maybe just take a pixel char map
- Modulations should work with canvas
- Uses locations for 2d plane rendering via projections
- Reference plane stitching

DMD Menu

- Highlighted item renders with a bar behind it and inverted colors
- Ability to render an image larger than what's available and "crop" it to a visible area
- Why is Renderable so weird?
- Frame offsets don't seem to factor into border widths
- Some kind of screen transition (z+, "plays" over top and goes away -- maybe a separate system?)
- Play sounds on inc/dec/select/back
- Only show menu if door is open

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
