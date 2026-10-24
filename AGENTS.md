# Agent Guidelines

## Systems

### Public system methods take `ServiceContext`

A public method on a system is called by other systems, so it must take `&ServiceContext`, never `&SystemContext`.
A `SystemContext` is a `ServiceContext` plus the handle of the current system, so passing one to another system
hands it the caller's identity: events it emits, cues it schedules, and lookups it does would all act as the caller.

The system builds its own `SystemContext` from its handle:

```rust
pub fn eject(&mut self, ctx: &ServiceContext) {
  let ctx = &ctx.for_system(self.handle);
  // ...
}
```

Private methods may take `&SystemContext`. When calling a public method from inside the system, convert with
`ctx.into()`. See the docs on `ServiceContext` in `frontbox/src/systems/service_context.rs`.

### Events and cues

- Cues a system sends to itself are private and live in the same file as the system.
- Public events for a module are centralized in that module's `mod.rs`, not spread across system files.
- Only halt an event with an interrupt when there is a specific reason it must not reach other systems. A halted event
  is invisible to everything downstream, including systems that are hard to anticipate (audits, switch hit counts, the
  console), so the default is to let it through.

## Ball Management (`frontbox-pinball/src/ball_management`)

- `TroughSystem` is the only system that reads trough switches. Other systems ask it (`occupancy()`,
  `read_switch_occupancy()`) and never read those switches themselves.
- Ball counts are derived from the trough's reported occupancy, not kept as running counters that drift when an event
  is halted, an eject fails, or a drain is missed. Consumers read `BallsDrained.still_in_play` rather than keeping their
  own counts.
