System

- Fix the drop target example, as delayed was removed (use a timer)
- Make machine bridge system deconfigure drivers on shutdown
- Bring back plugin which can register things like operator configs
- Should Timers be integrated into the Tickable/Timeline ecosystem? (probably)

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
- Pin2DMD support
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
