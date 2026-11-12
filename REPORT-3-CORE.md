# Report 3: Core architecture (run loop and systems)

Scope: `frontbox-core/src/app` (run loop), `systems` (containers, cues, routines, states, contexts), and the machine
layer as far as it affects the loop (`machine_system.rs`, `neuron/machine.rs`, `serial_interface.rs`,
`provided/watchdog.rs`). As asked, this looks only for runtime problems and worthwhile optimizations, not style, and
proposes no restructuring.

Everything was found by reading. One item (A3) was also confirmed by running the cue code in a scratch program
outside the repo.

## Summary

The design is sound and I'd leave its shape alone. In particular, the rule that every mutation of the system tree
(spawn, despawn, activation, cues, states, routines) goes through the message queue and takes effect in queue order
is what makes handlers safe to write, and routines and states are built consistently on it.

What I'd fix, most important first:

| # | Finding | Effect at runtime | Size of fix |
|---|---|---|---|
| A1 | Game time is counted in ticks, not measured | Timers, cues, and animations run slow, more so under load | Small |
| A2 | The watchdog renewal depends on A1 and waits behind other serial traffic | 48V can be cut during a stall | Small |
| A3 | Cue timelines drop a point that lands in the same tick as the one before | A cued event never arrives | Small |
| A4 | `on_tick` and `on_render` alternate per system | LEDs and DMD show every change one tick late | Tiny |
| A5 | Type lookup breaks if a system is spawned before an old one of its type despawns | `ctx.systems.get::<T>()` returns `None` for a running system | Tiny |
| A6 | Shutdown skips inactive systems and drops queued messages | `on_despawn` work doesn't happen | Small |
| A7 | A panic in any handler, including a `RefCell` double borrow, ends the process | Game dies mid-ball | Small to medium |
| B1 | Every message is `Debug`-formatted to a string | Wasted work on the hottest path | Tiny |
| B2 | Every event is serialized to JSON, tracers or not | Same | Tiny |
| B3 | LED traces travel through the run loop's queue | Load on the loop in debug builds; likely related to the console slowdown | Small |

## A. Runtime problems

### A1. Game time is counted, not measured

`handle_system_tick` passes `base.system_interval` as `delta` to every `on_tick`, whatever time has really passed.
Three things make real time run ahead of that count:

- The ticker task sends on a `watch` channel. If the loop is busy for more than one interval, the sends collapse into
  one `changed()` and the missed ticks are gone.
- The interval uses `MissedTickBehavior::Delay`, so each late tick pushes all later ticks back.
- A timer never wakes early, only on time or late. With `Delay`, every tick's lateness accumulates.

So the count is always behind real time by a little, and by a lot after any stall. Everything built on `delta`
inherits it: cues, `LedPlayback`, ball-save timers, the minutes-on metric. A 30 second timer takes 30 seconds plus
all lateness in that span. It also means light shows can't stay in step with audio, which runs on a real clock
(`TODO.md`: "proper BPM sync between music+scenes").

**Fix.** Keep an `Instant` in the run loop and pass the measured elapsed time as `delta`, clamped to a few intervals
so a long stall doesn't arrive as one enormous step. Nothing else changes; systems already take `delta`. With
measured deltas, the `Synchronize` event (which `TODO.md` questions) is only needed to align tick phase, not to
correct time.

A related note: the default `system_interval` is 83 ms (12 per second) and lotko uses 67 ms, while the `System`
docs say ticks are "typically 30-60Hz". Cue timing is quantized to this, and so is LED frame rate (Report 1, 5.4).

### A2. The watchdog renewal has two weak links

The chain is: `WatchdogSystem` starts a `Cue::Forever(interval - 250ms)`; the cue fires in `on_tick`; the system
calls `MachineSystem::ping_watchdog`; that queues `MachineMessage::WatchdogPing`; the machine task sends it to the
board.

Your earlier scoop-eject investigation found the watchdog cutting 48V when renewal raced the firmware timer. The
250 ms margin addressed that race, but two ways to exceed the margin remain:

1. **The cue runs on counted time (A1).** Any stall of the run loop delays the ping by the full stall, and the
   margin has to absorb tick quantization too (up to one interval, 67 to 83 ms). The one stall I can point to today
   is `DmdSystem::on_render`, a synchronous USB write with a 2 second timeout (Report 2, section 2). Decoding an image
   via `Content::picture` inside a handler would be another; its doc comment warns against that.
2. **The ping waits in line.** The machine task has one FIFO queue for both serial ports. A `WatchdogPing` sits
   behind whatever was queued first, and `Command` messages each wait for a response for up to 100 ms
   (`COMMAND_TIMEOUT`), 125 ms for a switch refresh. Three slow commands ahead of a ping use up the margin. Driver
   reconfiguration sends one per driver.

Stalling the run loop *should* trip the watchdog eventually; that is its purpose. The problem is that 250 ms of
margin is small against either cause.

**Fixes**, all small and independent:

- Renew on measured time: in `WatchdogSystem::on_tick`, ping when `last_ping.elapsed()` passes the renewal interval.
  Still tied to the loop being alive, no longer tied to tick counting.
- Give the ping priority in the machine task: its own channel, polled first with `biased;` in the `select!`.
- Size the margin as a proportion (renew at half the watchdog interval) and not a fixed 250 ms.
- `interval - Duration::from_millis(250)` panics if someone configures a watchdog of 250 ms or less; use
  `saturating_sub` or validate at boot.

### A3. Cue timelines drop events (confirmed)

In `CueAccumulator::accumulate`, a `Timeline` advances every remaining point each tick but only reports the first
one. If the second point also completes in that tick, its `completed` flag is set while it is still second in line.
On the next tick it becomes first, `accumulate` sees it's already complete and returns `false`, and its signal is
never delivered.

I ran this outside the repo with a 67 ms tick and points at 3000 ms, 3005 ms, and 4000 ms. The signals delivered
were the first and third. Any two points closer together than one tick behave this way, and at 67 to 83 ms per tick
that is not an unusual spacing for a light or sound cue list. `from_points` panics on exactly equal times, so the
limitation was known in that form; near-equal times fail silently.

If the dropped point is the last in the timeline, the accumulator also never reports a completed cycle again, so
`SystemContainer::on_tick` never removes it (a small leak until the system despawns).

**Fix.** Have a timeline deliver every point that has come due, in order: keep one elapsed time for the timeline and
pop points while `points.first().at <= elapsed`, calling `on_event` for each. That also removes the need for the
duplicate-time panic.

### A4. Render runs before most systems have ticked

`handle_system_tick` calls `on_tick` then `on_render` on each system in turn:

```rust
apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
  system.on_tick(tick_duration, ctx);
  system.on_render(ctx);
});
```

The doc on `System::on_render` says it runs "since on_tick processed all state needed for rendering", which needs
two passes: all ticks, then all renders. As written, `LedSystem` (spawned early) renders before any system spawned
after it has ticked, so every `LedPlayback::apply` and every `declare_leds` made in `on_tick` reaches the hardware on
the *next* tick. At 67 to 83 ms that is a visible lag between a switch hit and its light, and between light and
sound. The same holds for `DmdSystem`.

**Fix.** Two calls to `apply_to_systems`, one for ticks and one for renders.

### A5. Type lookup is lost when same-type systems overlap

`Systems::insert` writes `type_to_id[type] = new_id`. `Systems::remove` looks up the removed system's type and
removes `type_to_id[type]` unconditionally. So if a group briefly holds two systems of one type:

1. old `T` (id 1) is running;
2. `spawn(T)` adds id 2, and the mapping now points at 2;
3. old `T` despawns; the mapping for `T` is removed, although it pointed at 2.

Afterwards `ctx.systems.get::<T>()` returns `None` and routines skip `T` as "missing", while the new system keeps
receiving events and ticks, which makes this confusing to diagnose. The natural way to hit it is a system that
spawns its successor and then calls `despawn_self` (`replace_self` is safe because it despawns first).

**Fix.** In `remove`, clear the type mapping only if it points at the id being removed. Separately, decide whether a
second system of a type in one group should be rejected with a warning at spawn, since the docs describe one
instance per type per group.

### A6. Shutdown

Three things in the sequence after the loop breaks:

- `on_despawn` is called through `apply_to_systems`, which skips inactive systems and inactive groups. A system that
  saves or releases something in `on_despawn` doesn't get to if it happens to be inactive. A normal despawn
  (`despawn_system`) calls `on_despawn` regardless of activity, so shutdown is the odd one out.
- Systems are despawned in spawn order. `MachineSystem` is first, so it resets the expansion boards and disables
  drivers before the other systems' `on_despawn` run; anything they send to hardware arrives after the reset.
  Reverse order is the usual choice.
- The loop has stopped reading the queue, so anything `on_despawn` handlers emit or set (events, states, despawn
  messages) is dropped. Hardware commands are unaffected since they use the machine channel directly.

### A7. A panic anywhere ends the game

All systems run on one task with no isolation, so a panic in any handler unwinds out of `run`. The watchdog then
expires and the board removes power, which is the safe outcome for coils. For a machine on location, though, one bad
`unwrap` in a rarely used mode ends the game in progress.

One class of panic comes from the framework itself. `ctx.systems.get::<T>()` uses `RefCell::borrow_mut`, and the
loop holds a mutable borrow of the system whose handler is running. So these panic with "already borrowed":

- a system looking itself up by type;
- system A calling a public method on B (the pattern in `AGENTS.md`) which looks up A, directly or through a third.

These only fail on the code path that makes the cycle, which may be rare.

**Fixes**, in increasing size:

- Use `try_borrow_mut` in `Systems::get_by_type` and `get_by_id`. On failure, log an error naming both systems and
  return `None`, which callers already handle. That turns a crash into a logged no-op.
- Wrap each handler call in `catch_unwind`. On a panic, log it, despawn that system, and carry on. The container
  already centralizes every call into a system, so this is one place.
- A panic hook that tries to send `WatchdogClear` and blank the LEDs, so a crash leaves the machine dark, not frozen.

### A8. A serial disconnect leaves the game running without switches

`TODO.md` already lists USB reconnect handling. For the record, the current failure is quiet: `Neuron::run` selects
on `io_port.read_event()`, which returns `None` on a read error. That disables the branch until the next machine
message, when the read is retried and fails again. So the game carries on with no switch input, logging one read
error per outgoing message, and nothing tells the systems. Until reconnect exists, emitting an event or shutting
down when a port dies would make the state visible.

## B. Optimizations

### B1. Every message is formatted to a string

At the top of the message branch in `run`:

```rust
let cmd_text = format!("{:?}", command);
```

This runs for every message, and the string is used only if handling took over 1 ms. `AppMessage`'s `Debug`
includes the full contents of some variants: `SyncSwitchStates` formats every switch state, and
`TracerEvent(LedsRGBChange)` formats every `(index, color)` pair in the batch. In a debug build, where the LED
traces are enabled and formatting is slow, this is real work on each message.

**Fix.** Format only inside the slow branch. The message has been moved by then, so capture something cheap first
(a `&'static str` variant name, plus the event's `type_name` for the event variants).

### B2. Every event is serialized to JSON

`emit_event` builds a `TraceEvent::Event` with `event_box.try_json()` before `TracerSenders::send`, which then clones
it once per tracer. With no tracers attached the JSON is built and dropped. `SwitchClosed` and `SwitchOpened` carry a
cloned `Switch`, so each switch event pays for a clone and a serialization.

**Fix.** Skip the work when there are no tracers, or pass `TracerSenders` a closure that builds the event on demand.

### B3. LED traces go through the run loop

In debug builds, `MachineSystem::set_leds` sends `AppMessage::TracerEvent(LedsRGBChange)` to the run loop's own
queue, where it waits its turn, gets formatted (B1), and is forwarded to each tracer's unbounded channel. With
around 170 LEDs animating, that is several messages per tick that exist only to be forwarded.

This lines up with the open console slowdown: both hops are unbounded queues, so a consumer that falls behind only
shows as growing delay, never as an error. I can't tell from reading whether that is the cause. What would tell:

- log `app_receiver.len()` and each tracer queue's depth at a slow rate (once a second), not only at trace level;
- give `MachineSystem` the tracer senders so LED traces skip the run loop entirely;
- for LED frames specifically, keep only the latest state per LED when the consumer is behind. The console hub
  already batches on its side; the queue before it doesn't.

### B4. Activity checks

`apply_to_systems` calls each system's `is_active` on every event and every tick, and builds a `SystemContext`
(eight small structs) per system per call. This is fine at current scale. It becomes worth attention only if
`is_active` implementations do real work. `DmdMenuSystem::is_active` looks up a switch by name on every call, for
example. No change suggested; just keep `is_active` cheap.

## C. Things I checked and would leave alone

- **Queue-ordered mutation.** Handlers can't change `Groups` directly, so iteration in `apply_to_systems` is safe
  with no copying, and "takes effect in queue order" is one rule for spawn, activation, cues, states, and routines.
- **Routines.** Pausing with an inactive origin, steps counted as complete when a participant despawns or
  deactivates, abandoning when the origin is gone, and delivering `RoutineCompleted` to the origin first are all
  handled in `routines.rs`, with a sizable test file beside it (which I didn't audit). The one-per-type limit is handled by the caller where it matters
  (`pending_drains` in `competitive_game.rs`).
- **States.** Ownership and clearing on despawn are consistent with the rest.
- **Config saving** runs on a background task and coalesces writes, so it doesn't block the loop.
- **Switch handling.** State is updated before the event is emitted, so handlers see current state; events queued
  during a serial query are flushed in order before a state sync.
- **`select!` fairness.** Branch choice is random, so neither messages nor ticks can starve the other.

Two small traps that aren't active problems:

- `impl System for Box<dyn SpawnableSystem>` and `for Box<dyn ChildSystem>` forward only four of the nine trait
  methods (`on_render`, `on_routine`, `is_active`, `on_deactivate`, and `on_reactivate` fall back to defaults).
  Today the containers upcast the box directly, so these impls aren't used on the normal path. Passing an
  already-boxed system to `spawn` would use them, and would silently lose those five handlers. Forward all nine or
  remove the impls.
- A cue whose interval is shorter than one tick fires once per tick and its `elapsed` grows without bound. Harmless,
  and it goes away with measured time and the timeline fix.

## Suggested order

1. A4, A5, B1, B2: each a few lines, no behavior risk.
2. A3: contained in `cue.rs`, with the existing cue tests to extend.
3. A1, then the A2 fixes that build on it. Worth doing together and watching on the machine, since every timer gets
   slightly faster.
4. A7 (`try_borrow_mut` first), A6.
5. B3, when you next pick up the console slowdown.
