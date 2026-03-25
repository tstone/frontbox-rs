# Frontbox

## Overview

Frontbox is a homebrew arcade framework built for [FAST Pinball](https://fastpinball.com/) hardware, designed around an actor-like constructs called "Systems", which send send and receive signal.

> [!WARNING]
> Frontbox is in active, pre-release development with unstable APIs

### Features

- **Modular**: Almost every facet of machine operation can be replaced
- **Lightweight**: Built in [Rust](https://rust-lang.org/) to run on minimal hardware
- **Coherent**: Limited number of abstractions and consistent architecture throughout
- **Dynamic**: Flexible animation + accumulation system that applies to just about everything
- **Retro**: Pin2DMD and NeoSeg\* (alpha numeric) display support out of the box
- **Immersive**: Sound system with automatic music ducking\*

\* = Coming Soon

## Guide

### Systems

The heart of Frontbox is a `System`. Almost everything is a System: game modes, credit modes, sound mixer, even the display. Systems interact with the world through `Signals`. Systems are just Rust structs, which can manage their own state and be extended with private functions. They have a handful of callback type methods, including general lifecycle `on_startup` and `on_shutdown` handlers.

```rust
struct Example {
  private_data: u64,
}

impl System for Example {
  fn on_startup(&mut self, ctx: &mut Context, _systems: &Systems) {
    // <do cool stuff here>
  }
}
```

Systems include four lifecycle handlers:

- `on_startup`
- `on_deactivate`
- `on_reactivate`
- `on_shutdown`

#### Startup

Systems can given on startup, and will be started automatically, or dynamically spawned at runtime. Likewise, running systems can be despawned or replaced.

```rust
// Start a new system
ctx.spawn_system(Example::new());

// Stop the current system and immediately spawn a replacement
ctx.replace_self(Example::new());

// Just stop the current system
ctx.despawn_self();
```

#### Types of Systems

- `System` - Plain vanilla system which can be started on boot
- `SpawnableSystem` - System which can be dynamically started at runtime. Must be `Send + Sync` compatible
- `ChildSystem` - System which can be managed within a group (see "System Groups" below). Must implement `Clone`.

### Signals

Frontbox systems primarily interact with events.

|                 | Event                                     | Cue                                                          |
| --------------- | ----------------------------------------- | ------------------------------------------------------------ |
| **Description** | A signal broadcast to all Systems         | A signal that a system can send to itself, usually scheduled |
| **Scope**       | Multi-producer, multi-consumer            | Single-producer (self), single-consumer (self)               |
| **Interrupt**   | Can be interrupted - `register_interrupt` | Can be cancelled - `cancel_cue`                              |

Systems can implement a handler per signal type.

```rust
impl System for Example {
  // handler for broadcast events
  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context, systems: &Systems) { }

  // handler for scheduled cues
  fn on_cue(&mut self, cue: &dyn Signal, ctx: &mut Context) { }
}
```

Signals are handled by attempting a downcast into the expected type.

```rust
impl System for Example {
  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context, systems: &Systems) {
    // detect if the event is of type `SwitchClosed`
    if let Some(e) = event.downcast_ref::<SwitchClosed>() {
      log::debug!("Switch {} was closed!", e.name);
    }
  }
}
```

Signals are both something that the framework provides (e.g. switch open/closed) and something that can be defined by the end user. The only requirement is that values be thread safe (`Send + Sync`).

```rust
// Signals can simply be a body-less struct representing a typed thing
pub struct MyCustomThing;

// Signals can also contain data
pub struct MyCustomThing2 {
  pub prop1: u8,
  pub prop2: String,
}
pub struct MyTupleLikeThing(i8, i8);
```

### Events

Events are signals which are broadcast to to all systems. While it's technically possible for every system to emit every event, in practice typically only a small handle of systems emit a particular event.

```rust
ctx.emit(MyCustomThing2 { prop1: 4, prop2: "example".to_string() });

// ...

impl System for Example {
  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context, systems: &Systems) {
    if let Some(custom) = event.downcast_ref::<MyCustomThing2>() {
      log::debug!("Custom thing happened with {}, {}", custom.prop1, custom.prop2);
    }
  }
}
```

##### Event Layering

Sometimes systems receive lower level events (e.g. switch state changed) and process them into higher level events. These higher level events themselves get processed into game level events.

For example...

- The framework might emit a `SwitchClosed` event
- The `Trough` system interprets this and emits `TroughOccupancyChanged` and possibly `TroughFull`
- These trough level events are received by a game manager that may emit `PlayerTurnEnding`.

### Cues

Cues are signals a system can send to itself. There are four primitive types of cues:

1. **Once** -- Cue happens exactly once, after a given amount of time has elapsed
2. **Times** -- Cue happens N times, with an interval in between
3. **Loop** -- Cue happens until canceled, with an interval in between
4. **Now** -- Cue happens immediately, once

```rust
struct SomethingImportant(u8);

impl System for Example {
  fn on_startup(&mut self, ctx: &mut Context, _systems: &Systems) {
    // setup the cue
    ctx.cue(
      Cue::Times(3, Duration::from_secs(3)),
      SomethingImportant(100)
    )
  }

  fn on_cue(&mut self, cue: &dyn Signal, ctx: &mut Context) {
    // what to do when the cue happens (in this case, 3 times)
    if let Some(v) = cmd.downcast_ref::<SomethingImportant>() {
      log::debug!("Something important: {}!", v.0);
    }
  }
}
```

#### Handles

Creating a cue returns a handle that can later be used to cancel it.

```rust
let handle = ctx.cue(SomethingRather, Cue::Forever(Duration::from_secs(1)));

ctx.cancel_cue(handle);
```

#### Timelines

Sometimes it's easier to express things as a linear timeline. The same example as above could also be expressed as...

```rust
ctx.cue_timeline(AllDone, Timeline::new()
  .cue_at(Duration::from_secs(3), SomethingImportant(5))
  .cue_at(Duration::from_secs(6), SomethingImportant(50))
  .cue_at(Duration::from_secs(9), SomethingImportant(500))
);
```

With timelines, there is not only a cue that happens for each node of the timeline (with a specific value), but also a cue for the entire timeline completing. Canceling a timeline cancels all remaining cues within it.

#### Cycling & Flashing

Cycling through a set of states is a common occurrence in pinball. For example, flashing is in fact the cycling of two values.

```rust
// note the use of `signals!` here rather than `vec!`
ctx.cue_cycling(signals![
    On("example"),
    Off("example"),
  ],
  Cue::Forever(Duration::from_secs(1))
);
```

Cycling works by rotating through the list of values each time the cue is complete. Think of it like a normal cue that just keeps rotating which signal is emitted, in order. In the example above, 1 second would elapse, then `On("example")` would be cued. Another second would elapse and `Off("example")` would be cued. Another second would elapse and `On("example")` would be cued, and so on.

### Generic Signals

In some cases, particularly with cueing, it might be a bit tedious to create a custom type for every little thing that happens. Generally this is preferred, but for insignificant situations the framework provides a few pre-built signals that can be used as one-offs:

- `&'static str` - It's possible to use a static string as a signal
- `Action`
- `Anonymous`
- `On` / `Off`

### Animations

Animations are a fundamental part to any arcade machine and especially to pinball. Whereas a `Cue` is about an event in time that a system handles, an animation is about a value that changes over time (though not necessarily bound to time). It's useful to establish first what exactly an animation is, before demonstrating how to use it.

Animations describe "how does this value change over an accumulated amount?" Usually the thing being accumulated is time.

```rust
let anim = Tween::new(
  Duration::from_secs(1),
  Curve::Linear,
  vec![0, 100],
  AnimationCycle::Once
);

log::debug!("Current value: {}", anim.sample());
// => "0"

anim.accumulate(Duration::from_millis(500));
log::debug!("Current value: {}", anim.sample());
// => "50"
```

This example describes how a value will start at `0` and end up at `100` over the duration of 1 second. The current value of the animation can be read by sampling it (`.sample()`). Calling `tick` causes time to march forward. Sampling the value of changed time will yield a new value.

#### Ticking Forward

Animations are actually built on a lower level trait called a `Accumulator`. Accumulator are, as the name implies, accumulators of values. When used with `Duration` they accumulate time.

```rust
acc.accumulate(Duration::from_millis(100));
log::debug!("Is complete? {}", acc.is_complete());

acc.reset();
```

Systems have an `on_tick` handler, invoked by the framework, that marches forward based on the framework frequency much like all game frameworks. This internal tick is separate from hardware event handling, which is done in real time. Inactive systems do not tick forward (see "Active" section).

```rust
impl System for Example {
  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.anim.tick(delta);
  }
}
```

#### Accumulation

While in the example above the animation was accumulating time by way of `Duration`, it's possible to accumulate anything that is, well, accumulatable. There are a few trait restrictions, like it must have a default value and be comparable (`PartialOrd`), summable, etc. but beyond that any accumulatable value can be accumulated.

This means that animations work, not just on time, by for integers that represent hit counts or switch counts. For example, to change the color of LED based on how many time a spinner has spun, an animation can be used for this.

```rust
// Require 100 hits, animating a from yellow to red
self.anim = Tween::new(
  100, // target
  Curve::Linear,
  vec![Color::yellow(), Color::red()],
  AnimationCycle::Once
);


fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context, systems: &Systems) {
  if let Some(e) = event.downcast_ref::<SwitchClosed>() {
    match e.name {
      switches::SPINNER => {
        let result = self.anim.accumulate(1);
        if result.completed_just_now {
          // do something
        }
      }
    }
  }
}

// elsewhere the animation value can be used to set the LED color

ctx.command(SetLed(leds::SPINNER_LANE, self.anim.sample()));
```

### Context

Each handler receives a reference to `Context`. As this guide has shown, it's through Context that access several features is provided, including:

- Register cues and interrupts
- Emit events
- Access to the global store (below)

#### Global Store

The other half of `Context` is the global store. All systems have read/write access to a shared state bucket.

> _**BZZT**_ We interrupt this guide for a special broadcast. Typically global, mutable state is a poor design choice, easy to abuse, and a bringer of monolithic mess. However, in architecture design, everything is a trade-off. A fully signal-based approach brings a few hefty requirements: (1) systems must always be active and receiving all events; and (2) systems must implement internal buffering/caching to infer current state from a stream of signals. A corollary to (1) is that (3) systems typically always need some kind of "bootstrap" process if they come online in the middle of execution to get a view of the current state.
>
> With pinball in particular, these requirements don't mesh well with machine operation: Displays need shared access (e.g. current player, score, extra ball status, etc.); Multiplayer games means there is constant switching of which set of listening systems are active.
>
> To implement a fully signal-based architecture, making systems for the inactive player listening and building up a current view, creates more complexity and opportunity for weird bugs than just using global mutable state. Frontbox adopts a trade-off: global mutable state, while posing some danger, is the simpler and less error prone approach.

> [!TIP]
> Use the global store only for (1) data that needs to be displayed or (2) read-on-demand reference data

The global store works based on _type_. Only one instance of a given type can be stored in the global store at once. Inserting a value of a type overwrites any previous values.

```rust
let value: Option<A> = ctx.get::<A>();
```

`Context` provides several access methods:

- `has::<T>` - returns `bool` if `T` exists in the global store
- `is::<T>(value)` - returns `bool` if value of type `T` is equal to `value`. Requires `T`implements`PartialEq`.
- `get::<T>` - returns `Option<&T>`
- `get_mut<T>` - returns `Option<&mut T>`
- `get_or_default::<T>` - returns `&T` or `&T::default()`
- `get_or_insert::<T>` - returns `&mut T` or `&mut T::default()`, inserting it automatically if not present
- `insert::<T>` - inserts `T` into the store
- `remove::<T>` - removes `T` into the store
- `expect::<T>` - returns `&T`, panics if it doesn't exist
- `expect_mut::<T>` - returns `&mut T`, panics if it doesn't exist

##### States

A useful pattern with global state is storing `enum` values, allowing a kind of distributed state machine. Several framework systems use this approach.

```rust
pub enum BossFightPhase {
  CentralHit,
  HitWithFireballs,
  HahaYouThoughtHeWasDead,
}

if ctx.is(BossFightPhase::HitWithFireballs) {
  // ...
}
```

> [!TIP]
> Use enums stored in global context as state machines

### Active

By default, all systems spawned are active. Systems can be despawned, which removes them entirely, but sometimes it's necessary to keep a system around, having it automatically become active in certain situations. Frontbox supports this feature by way of the `is_active() -> bool` handler.

If `is_active` returns `false`, the framework will by skip all other handlers (the ones starting with `on_*`). Within `is_active`, only read access to `self` and `Context` is provided.

```rust
// a common 'active toggle' is that a system is only active when at least one ball is in play for a player
impl System for Example {
  fn is_active(&self, ctx: &Context) -> bool {
    ctx.is(CurrentPlayerTurnState::Active)
  }
}
```

### Multimedia

#### LEDs

LEDs are declared through commands.

```rust
ctx.command(SetLed(leds::LEFT_LANE, LedState::On(Color::red())));
```

> [!WARNING]
> This is an evolving feature

#### Sounds

The default sound system works through commands.

```rust
ctx.command(PlaySFX(path));
ctx.command(PlayMusic(path));
ctx.command(CrossfadeMusic(path, Duration::from_millis(500)));
ctx.command(FadeOutMusic(Duration::from_millis(150)));
```

> [!WARNING]
> This is an evolving feature

### Operator Config

TODO: The implementation is minimal and this needs to be flushed out some more.

### Niche Features

#### Event Interrupts

Sometimes there are cases where the normal flow of operation needs to be halted. For example, if a player drains while ball save is active, this would _normally_ emit an event that the player has drained and the turn is over. In these cases it's necessary to allow a system to override this behavior. This happens by way of event interrupts.

Systems can register themselves as an event interrupt. Interrupt registration requires a priority. The framework will interrupts in priority order (highest first). This allows, for example, a temporary start-of-ball ball save to take precedence over an extra ball or outlane ball save.

Event interrupts can be applied to any event within the system.

```rust
fn on_startup(&mut self, ctx: &mut Context, _systems: &Systems) {
  ctx.register_interrupt::<TurnEnd>(100); // 100 is the priority
}

fn on_interrupt(&mut self, event: &dyn Signal, ctx: &mut Context) -> InterruptResult {
  // interrupt handlers must return a result
  InterruptResult::Continue // or InterruptResult::Halt
}
```

#### System Groups

System groups are a feature that allows a group of systems to be toggled active or inactive together. This is independent from the `is_active` handler, which is a per-system feature. An entire group can be made inactive, which automatically makes each system within that group no longer receive signals. The systems could still be declaring themselves as active. Within a group, the active/inactive nature is actually a combination `group is active && system is active`.

This feature is primarily used by the framework to implement automatic switching of systems based on active player, but it likewise could be used to implement scene switching.

Systems spawned into a group must implement `ChildSystem`, which requires that they be `Clone + Send + Sync`.

```rust
const group_name: &'static str = "example";

// Start an entire group of systems
ctx.spawn_system_group(group_name, vec![/* list of systems */]);

// Groups start deactivated by default
ctx.activate_system_group(group_name);
ctx.deactivate_system_group(group_name);

// The entire group can be despawned. All `on_shutdown` handlers will be invoked for child systems
ctx.despawn_system_group(group_name);
```

### Plugins

TODO: The implementation is minimal and this needs to be flushed out some more.

### Bootstrapping

Frontbox provides an `App` interface which is the root of the framework. With `App`, hardware can be defined and booted.

```rust
App::boot(
  BootConfig {
    platform: FastPlatform::Neuron,
    io_net_port_path: "/dev/ttyACM0",
    exp_port_path: "/dev/ttyACM1",
    ..Default::default()
  },
  io_network,
  expansion_boards,
).await
```

This returns an app builder, which can continue to have systems, plugins, operator configuration, systems, and similar chained onto it before finally running with the initial set of systems.

```rust
App::boot(
    BootConfig::default(),
    io_network,
    expansion_boards,
  )
  .await
  .systems(vec![ ... ]) // add initial systems
  .plugin(CompetitivePlay::default()) // add plugins
  .watchdog_tick(Duration::from_millis(1250)) // configure hardware interactions
  .operator_config_item("required_target_hits", ConfigItem::Integer {
    default: 5,
    min: 3,
    max: 7,
    ..Default::default()
  })
  .run() // call run to begin
  .await
```

#### Hardware Definition

##### I/O Network

The I/O network is defined using the `IoNetworkBuilder`. See [Defining Hardware]() guide for more details. I/O network devices can either associate a name with a pin, or can optionally provide a configuration. Configurations given here are automatically applied at startup.

```rust
pub mod switches {
  pub const LEFT_INLANE: &str = "left_inlane";
  pub const LEFT_OUTLANE: &str = "left_outlane";
}

pub mod drivers {
  pub const TROUGH_EJECT: &str = "trough_eject";
  pub const AUTOPLUNGER: &str = "autoplunger";
}

let mut io_network = IoNetworkBuilder::new();

io_network.add_board(
  FastIoBoards::io_3208()
    .with_switch(switches::LEFT_INLANE, 3)
    .with_switch_cfg(switches::LEFT_OUTLANE, 4, SwitchConfig {
      inverted: true,
      debounce_open: Some(Duration::from_millis(60))
    })
    .with_driver(drivers::TROUGH_EJECT, 0)
    .with_driver_cfg(drivers::AUTOPLUNGER, 1, PulseMode {
      trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
      initial_pwm_power: Power::FULL,
      ..Default::default()
    })
);
```

##### Expansion Network

> [!WARNING]
> Defining the expansion network is scheduled to be re-written to be more consistent with defining the I/O network and be more flexible around defining LED groups, ports, etc.

The expansion network is defined using the `ExpansionNetworkBuilder`. See [Defining Hardware]() guide for more details.

```rust
pub mod leds {
  pub const LEFT_INLANE: &str = "left_inlane";
  pub const LEFT_OUTLANE: &str = "left_outlane";
}

let mut exp_network = ExpansionNetworkBuilder::new();

exp_network.add_board(
  ExpansionBoard::fp_exp0061()
    .with_led_port(LedPort {
      port: 0,
      start: 0,
      led_type: LedType::WS2812,
      leds: vec![leds::LEFT_INLANE, leds::LEFT_OUTLANE]
    })
)
```

## Roadmap

1. Playing sounds/music
2. Accumulations
3. Expansion network definition re-write
4. LED system revamp: Use new cueing + command systems; support led groups
5. NeoSeg support

## Complete Example System

This system implements a basic pinball "mode". A target is illuminated and must be struck 3 times. Each hit grants 1000 points. After 3 hits, the target will begin flashing. The player has 20 seconds to hit it again for 25,000 points (hurry up shot). After 20 seconds or being hit a 4th time the mode resets.

- `SwitchClosed` event monitors the target's switch
- `ctx.set_timer` and `TimerComplete` event monitors the hurry up timer
- `self.hurry_up_active` and `self.hits` manage state
- `fn leds` sets the LED state for the framework to apply (declarative)

```rust
const HURRY_UP_TIMER: &'static str = "hurry_up";

struct TargetHitter {
  // current times this target has been hit
  hits: u8,
  // animation for bonus hit
  flash_anim: Box<dyn Animation<Color>>,
  state: TargetHitterState,
  // ids for target switch and LED indicator
  target_switch_id: &'static str,
  indicator_id: &'static str,
}

enum TargetHitterState {
  // waiting to get to the desired number of hits
  Building,
  // bonus hurry-up mode for extra points
  HurryUp
}

impl TargetHitter {
  pub fn new(target_switch_id: &'static str, indicator_id: &'static str) -> Box<Self> {
    Box::new(Self {
      target_switch_id,
      indicator_id,
      hits: 0,
      state: TargetHitterState::Building,
      flash_anim: InterpolationAnimation::new(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        vec![Color::black(), Color::red()],
        AnimationCycle::Forever,
      )
    })
  }

  fn reset(&mut self) {
    self.hits = 0;
    self.hurry_up_active = false;
    self.flash_anim.reset();
  }

  // Here's what happens when the target is it -- if the mode is in "hurry up"
  fn on_target_hit(&mut self, ctx: &Context, systems: &Systems) {
    let game_manager = systems.expect_mut::<GameManager>();

    match self.state {
      TargetHitterState::HurryUp => {
        game_manager.add_points(25_000, ctx);
        self.on_hurry_up_done();
      }
      TargetHitterState::Building => {
        self.hits = self.hits.saturating_add(1);
        game_manager.add_points(1000, ctx);

        if self.hits == 3 {
          self.hurry_up_active = true;
          cmds.set_timer(HURRY_UP_TIMER, Duration::from_secs(20), TimerMode::Once);
        }
      }
    }
  }

  fn on_hurry_up_done(&mut self) {
    self.reset();
  }
}

impl System for TargetHitter {
  fn on_event(&mut self, event: &dyn Signal, ctx: &mut Context, systems: &Systems) {
    if let Some(event) = event.downcast::<SwitchClosed>() {
      if event.switch.id == self.target_switch_id {
        self.on_target_hit(ctx);
      }
    }
  }

  fn on_timer(&mut self, name: &'static str, _ctx: &mut Context) {
    if event.name == HURRY_UP_TIMER {
      self.on_hurry_up_done();
    }
  }

  fn leds(&mut self, delta_time: Duration, _ctx: &Context) -> LedStates {
    // show the flashing state if hurry up is active otherwise use a static color
    match self.state {
      TargetHitterState::HurryUp => {
        LedDeclarationBuilder::new(delta_time)
          .next_frame(self.flash_anim)
          .collect()
      }
      TargetHitterState::Building => {
        let color = match self.hits {
          0 => Color::yellow(),
          1 => Color::orange(),
          2 => Color::red(),
        }
        LedDeclarationBuilder::new(delta_time)
          .on(self.indicator_id, color)
          .collect()
      }
    }
  }
}
```

See [examples](frontbox/examples) or the [included plugins](frontbox-rs/tree/main/frontbox-turn-based/src) for more.
