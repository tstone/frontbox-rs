System

- SystemBundles
- System to automatically register drivers and unregister them on shutdown
- Make a nice way to easily add systems like watchdog and driver config applier (that moves out of App builder)
- Fix game points plugin -- how is player-specific state handled
- Figure out commands -- registered from plugins?
- Move app, store to top-level modules
- handle_event! doesn't seem to allow multiple
- Trough system
- Current ball index
- States (BallLocation, GameStartable) -- these now become enums in Store/World

Hardware

- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

Timers

- Timer tick seems to be broken in the drop targets example

DX

- Some kind of console runner that shows switch states and has a terminal/console (this needs to skip the command listening part... somehow)

Displays

- Trait or re-use System?
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

Websocket support

- Design it
