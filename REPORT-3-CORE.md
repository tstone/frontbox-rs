# Report 3: Core architecture (run loop and systems)

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

