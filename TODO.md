Before Show

- BUG: there seems to be a bug with Tween where occasionally after ~8 seconds it plays backwards (is it when it's accumulated the exact amount?)
- Defining an LED or strip (eg. `led(...)`, `led_strip(...)`) should be the same as when declaring them, `led` vs `named_led` is weird
- Add a multi-stop gradient to the declarations
- frontbox-sound multi-stem music support
- frontbox-sound loop point support
- Create a new frontbox-canvas crate
- combine DMD rendering + led canvas rendering
- standardize on image/Rgba<u8> for canvas
- LedSystem should maybe break away to be it's own crate? maybe animation too, and implement palette for HSL/color modifications
- Single channel flasher support
- NeoSeg support
- Auto plunger plugin has operator config for coil power/kick length
- OperatorConfig should be a separate System -- this will need a way for plugins to register data for systems that aren't yet started

Nice to Have

- Some kind of persistable storage (re-use Store, but add Deserialize requirement)
- Keep stats on coils fired, etc.

LEDs

- Implement binary versions of LED commands
- System groups can contain groups
- Keyframe animation -- specify "this value, at this point in time" -- like tween but adjustable time durations between
- has_tag/has_typed_tag should probably be a trait
- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

DX

- Figure out how to do step debugging on a live machine
