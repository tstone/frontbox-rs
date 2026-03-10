Hardware

- Add driver configure and trigger support for "hold" modes

Display

- Draft of websocket system
- Some kind of internal event bus that also projects to websockets? How to know what events get published out WS
- Mutations to store emit WS events and/or stores are accessible via WS

Timers

- Timer tick seems to be broken and running constantly. Example drop targets. Needs some unit tests

LEDs

- Make LED resolver something that can be changed dynamically at any time (or at least configured)
  - should LED resolver be per LED?
- Modulators + lenses -- Allow any property to be modulated
- Support Z-index
