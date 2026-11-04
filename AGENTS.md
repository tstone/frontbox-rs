# Agent Guidelines

## Code Layout

A file leads with its main type: the struct, then its `impl` blocks. Private helper types (internal structs, enums)
go below the main type's `impl`, never between the struct and its `impl`.

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

- Name events in the past tense (`BallSaved`), and routine payloads in the present progressive for what's going on
  while they run (`TurnEnding`).
- Say "complete" rather than "finish" for something that runs to an end, such as a routine or one of its steps
  (`RoutineCompleted`, `complete_routine_step`).
- Cues a system sends to itself are private and live in the same file as the system.
- Public events for a module are centralized in that module's `mod.rs`, not spread across system files.
- An event always reaches every active system; nothing can stop it. When systems need the chance to change what
  happens next (saving a drain, extending a turn), make that a routine they take part in, and have them halt it.

## Ball Management (`frontbox-pinball/src/ball_management`)

- `TroughSystem` is the only system that reads trough switches. Other systems ask it (`occupancy()`,
  `read_switch_occupancy()`) and never read those switches themselves.
- Ball counts are derived from the trough's reported occupancy, not kept as running counters that drift when an event
  is halted, an eject fails, or a drain is missed. Consumers read `BallsExitedPlay.in_play` rather than keeping their
  own counts.
- Ball management reports drains but doesn't decide what they mean. During a turn, the game manager runs the
  `BallDraining` routine for each drain. Systems that can save a drain (ball save, outlane saves) are its
  participants, and halt it when they save one; the app lists them in the order they get the chance.
- `BallsExitedPlay` includes drains that are then saved. Anything that should only react to drains that stand (a
  multiball ending) watches `RoutineCompleted<BallDraining>` for `RoutineCompletion::AllSteps` instead.
