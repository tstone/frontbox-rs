- Need to have a RandSystem that manages seed per game
- Redo ActionButtonEject(System) to use plunge lane events + accept LedProgram1ds
- Animations should be able to be specified as relative to the frame rate
- Establish (and document) consistent log targets
- Check why there is an IoBoard and IoBoards
- Audits: Keep stats on coils fired, etc.
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Stability: Robust handling for USB disconnects/reconnects
- Operator config changes should update HardwareValues automatically -- is this a system that listens to config change events?
- Clean up hardware exports
- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension
- Streamline animation curve choices

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

LEDs

- Because of the way LedProgram1d is "owned" by the system, the system has to hang around to let the effect complete. This makes system replacement unnecessarily difficult, and creates weird "lag" feeling in the software. Instead there could perhaps be a "LedProgramOneShot" system which works more play `play_sfx` where something like `play_effect` could be called it places through, then removes itself independent of the system starting it. This would need to be ServiceContext to keep it alive and that may result in overlapping declarations (perhaps LedSystem itself manages these and doesn't auto-remove on despawn?).
- Implement binary versions of LED commands
- Should "timeline" be renamed "keyframe"?
- LedQ::any naming is weird, because it's basically saying "all of these" but it's written like a query "any of these are true"
- Defining an LED grid like strip, but with rows/cols and serpentine directions -- ColorMatrix instead of ColorSequence?
- LedSystem should maybe break away to be it's own crate? maybe animation too, and implement palette for HSL/color modifications
- Single channel flasher support
- NeoSeg support
- combine DMD rendering + led canvas rendering

Nice to Have

- frontbox-sound multi-stem music support
- frontbox-sound loop point support

DX

- The web console needs help and probably a real SPA
