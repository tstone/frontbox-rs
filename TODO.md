Hardware

- When specifying driver mode, e.g. PulseMode, switches should be specified as &'static str, then mapped to their corresponding ID
- Improve the "activate this driver" behavior to make it clearer (ie. delcare them as off by default then activate at the correct time)
- Add driver configure and trigger support for "hold" modes

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
