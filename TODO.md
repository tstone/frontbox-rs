System

- Operator config should become a system
- Turn individual player system into a plugin + complete team/co-op side
- Make machine bridge system deconfigure drivers on shutdown
- Read/write Store values to disk on boot/shutdown
- Keep stats on coils fired, etc.
- Broadcast config changes
- trough system, auto plunger react to config changes and update driver config
- move hardware setup out of App
- System groups can contain groups
- Keyframe animation -- specify "this value, at this point in time" -- like tween but adjustable time durations between

Hardware

- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

DX

- Some kind of console runner that shows switch states and has a terminal/console (this needs to skip the command listening part... somehow)
- Debugging: Some kind of websockets console to see what's going on (could this be an app plugin?)

Sound (as a System)

- basic interface (play music, play sound, play sfx)
- kira implementation in a separate crate

Displays (as a System)

- FAST LED canvas
- NeoSeg support

LEDs

- Should LEDs be managed like sounds, `ctx.command(DeclareLedState(...))` ? -- probably, and a separate system/crate
- Move animation handling into on_tick
- LED renderer should use Context to store LED state
- Allow LEDs to be specified as a group, one name to talk to them all (e.g. for GI) maybe some kind of Into<LedGroup>
- Allow declarations to happen on groups as well
- Single color flasher support
- LED configuration at startup (mostly specify resolution behavior)
- Make LED resolver something that can be changed dynamically at any time `ctx.command(ConfigLed)`
- Modulators + lenses -- Allow any property to be modulated
- Support Z-index
