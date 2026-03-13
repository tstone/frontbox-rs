System

- Current ball index
- Trough system
- States (BallLocation, GameStartable)

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

- Allow LEDs to be specified as a group, one name to talk to them all (e.g. for GI) maybe some kind of Into<LedGroup>
- Allow declarations to happen on groups as well
- Single color flasher support
- Make LED resolver something that can be changed dynamically at any time (or at least configured)
  - should LED resolver be per LED?
- Modulators + lenses -- Allow any property to be modulated
- Support Z-index

Websocket support

- Design it
