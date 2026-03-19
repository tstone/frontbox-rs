System

- Split trough system and the like out to frontbox-pinball crate 
- How does the Game start know if the trough is full at the start w/o an event? (Context?)
- Auto plunger management system
- Fix the drop target example, as delayed was removed (use a timer)
- Make a system to deconfigure drivers on shutdown
- Bring back plugin which can register things like operator configs

Hardware

- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

DX

- Some kind of console runner that shows switch states and has a terminal/console (this needs to skip the command listening part... somehow)
- Debugging: Some kind of websockets console to see what's going on (could this be an app plugin?)

Displays (as a System)

- FAST LED canvas
- Pin2DMD support
- NeoSeg support

LEDs

- LED renderer should use Context to store LED state
- Allow LEDs to be specified as a group, one name to talk to them all (e.g. for GI) maybe some kind of Into<LedGroup>
- Allow declarations to happen on groups as well
- Single color flasher support
- Make LED resolver something that can be changed dynamically at any time (or at least configured)
  - should LED resolver be per LED?
- Modulators + lenses -- Allow any property to be modulated
- Support Z-index