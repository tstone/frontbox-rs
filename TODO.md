System
- Test going from attract mode to in game. Check if push_runtime works with +send+'static
- Custom events?

Hardware
- Dual wound flipper config

Display
- Draft of websocket system
- Some kind of internal event bus that also projects to websockets? How to know what events get published out WS
- Mutations to store emit WS events and/or stores are accessible via WS

LEDs
- Make LED resolver something that can be changed dynamically at any time (or at least configured)
  - should LED resolver be per LED?
- Maybe color should be it's own top-level feature (or separate crate?)
- Modulators + lenses -- Allow any property to be modulated
