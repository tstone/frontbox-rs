System

- Current ball index
- Need to figure out a better way to activate the Watchdog, as it's needed for low voltage drivers (start button)
- Trough system
- States (BallLocation, GameStartable)

Hardware

- Add driver configure supportr for 30 Delayed Pulse, 75 Pulse w/ Cancel, 78 Pulse Hold Extension

Display

- Draft of websocket system
- Some kind of internal event bus that also projects to websockets? How to know what events get published out WS
- Mutations to store emit WS events and/or stores are accessible via WS

Timers

- Timer tick seems to be broken in the drop targets example

LEDs

- Allow LEDs to be specified as a group, one name to talk to them all (e.g. for GI) maybe some kind of Into<LedGroup>
- Allow declarations to happen on groups as well
- Single color flasher support
- NeoSeg support
- Make LED resolver something that can be changed dynamically at any time (or at least configured)
  - should LED resolver be per LED?
- Modulators + lenses -- Allow any property to be modulated
- Support Z-index
