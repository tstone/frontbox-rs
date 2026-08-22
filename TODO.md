- Redo ActionButtonEject(System) to use plunge lane events + accept LedProgram1ds
- Animations should be able to be specified as relative to the frame rate
- Establish (and document) consistent log targets
- Check why there is an IoBoard and IoBoards
- Audits: Keep stats on coils fired, etc.
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Stability: Robust handling for USB disconnects/reconnects
- Add `by_location` to hardware lookup implementations + HardwareQuery
- Operator config changes should update HardwareValues automatically -- is this a system that listens to config change events?
- Don't export SystemContainer types in prelude
- Clean up hardware exports
- It's only a matter of time before passing around Context results in a system shutting down itself and accidentally shutting down it's caller
- Possible BUG: If Cue time is less than system tick time, what happens?

Canvas

- Modulations should work with canvas
- Uses locations for 2d plane rendering via projections
- Fill2d needs a perlin noise fill
- Reference plane stitching

DMD Menu

- Implement all sounds
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
