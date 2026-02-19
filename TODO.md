System
- Systems spawn into a scope, Machine, Runtime, Stack
- Test going from attract mode to in game. Check if push_runtime works with +send+'static

Hardware
- Dual wound flipper config

Display

- Draft of websocket system
- Some kind of internal event bus that also projects to websockets? How to know what events get published out WS

LEDs

- Some kind of laziness where an LED can be left on without being re-rendered (maybe this works by remembering the last set state and not repeating it)
- Make LED resolver something that can be changed dynamically at any time
  - should LED resolver be per LED?
- Maybe color should be it's own top-level feature (or separate crate?)
- Modulators + lenses -- Allow any property to be modulated
