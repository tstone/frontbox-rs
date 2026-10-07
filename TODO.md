- App should periodically re-ping the hardware to get the state of things (maybe?) -- this might already happen for switches
- Update the trough to periodically re-check the state of the switches and self recover if it is actually full

- ExpAddress is defined twice, once in fast-protocol and once in frontbox. The latter has "port". This needs to be resolved.
- So much is built on LedSystem that the framework should probably just start it up automatically.
- Need to have a RandSystem that manages seed per game
- Operator config changes should update HardwareValues automatically -- is this a system that listens to config change events?
- Trough needs to properly utilize jam sensor
- Establish (and document) consistent log targets
- Audits: Keep stats on coils fired, etc.
- Add driver configure support for 78 Pulse Hold Extension
- High scores are janky and require too much system switching (but how else to support multiple input methods?)
- Clean up hardware exports
- Streamline animation curve choices
- Animations should be able to be specified as relative to the frame rate
- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Stability: Robust handling for USB disconnects/reconnects
- Have a Claude skill that's ready to go for people (how can this not duplicate docs?)
- Shutdown should probably have some way of configuration an FnOnce instead of the current system that hard-codes linux-specific shutdown handling

Canvas

- Modulations should work with canvas
- Uses locations for 2d plane rendering via projections
- Fill2d needs a perlin noise fill
- Reference plane stitching

DMD

- DMD package probably needs to become frontbox-dmd since most of the code could support other DMDs
- Implement all sounds
- Animate right offset of section arrow when selected
- Transition left/right ease between sections
- Fancy: the selection box animates between vertical offsets
- Support a virtual DMD that renders to the console (maybe?)

LEDs

- While LedProgram makes things feel normal, it's still a little weird how sometimes things are still manually declared. Feels like there needs to be a unified API.
- Because of the way LedProgram1d is "owned" by the system, the system has to hang around to let the effect complete. This makes system replacement unnecessarily difficult, and creates weird "lag" feeling in the software. Instead there could perhaps be a "LedProgramOneShot" system which works more play `play_sfx` where something like `play_effect` could be called it places through, then removes itself independent of the system starting it. This would need to be ServiceContext to keep it alive and that may result in overlapping declarations (perhaps LedSystem itself manages these and doesn't auto-remove on despawn?).
- Really should revert name back to "effect" which is so much more natural LedEffect1d, LedEffect2d
- Implement binary versions of LED commands
- Should "timeline" be renamed "keyframe"?
- LedQ::any naming is weird, because it's basically saying "all of these" but it's written like a query "any of these are true"
- Defining an LED grid like strip, but with rows/cols and serpentine directions -- ColorMatrix instead of ColorSequence?
- LedSystem should maybe break away to be it's own crate? maybe animation too, and implement palette for HSL/color modifications
- Single channel flasher support
- NeoSeg support
- combine DMD rendering + led canvas rendering
- Expand named color library
- Maybe: Declare library colors with FAST and reference by ID

Sound

- frontbox-sound multi-stem music support
- frontbox-sound loop point support
- legacy hardware emulation? e.g. YM2610 FM

Console

- Initial state: configure switches to be latched by default (e.g. trough, captive ball)
- Macros: recordable + re-playable switch/coil sequences (can also reference other macros)
- Event sourcing/replay - dump per game (serialize to JSON) - record timestamp as a well
- LedEffect1d web based designer: uses an API to render actual colors
- LedEffect2d web based designer: same as above, but also uses LED config
