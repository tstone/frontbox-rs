System

- Trough system -- work out the complete plunge/play/drain cycle with a player
- Split trough system and the like out to frontbox-pinball crate
- Move configuring hardware to a system, make it also deconfigure drivers on shutdown
- When a system is despawned, it's command and interrupt registrations need to be removed
- Systems need to be able to declare some kind of dependency on what they require, so as to get a panic if it's missing
  e.g. `ctx.require_command::<SomeCommand>` to `ctx.require_event::<E>` which also means there needs to be a `ctx::provides_event<E>`.

Turn Based

- Current turn

Hardware

- Add driver configure support for 75 Pulse w/ Cancel, 78 Pulse Hold Extension

Timers

- Timer tick seems to be broken in the drop targets example

DX

- Some kind of console runner that shows switch states and has a terminal/console (this needs to skip the command listening part... somehow)

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

Websocket support

- Design it -- as a system
