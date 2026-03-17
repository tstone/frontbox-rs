# Frontbox

Frontbox is a Rust native framework for running arcade machines -- particularly homebrew pinball -- built on FAST pinball hardware.

> [!WARNING]
> Pre-alpha work in progress

### Overview

Frontbox is both an "ECS light" and "actor light" framework.

It's "ECS light" in that there is a shared, global store which all systems can interact with. Unlike traditional ECS, only entity singletons are stored in the "world", and there are no components. This effectively means that exactly one copy of a type can be stored at a time ("anymap"), operating like Bevy's `Resource` type. Like ECS, systems can emit events which are broadcast to all systems, and declare under what conditions they are active.

Frontbox is also "actor light" in that systems can operate as actors, holding internal state private to them and registering commands which they send to each other. Command routing is performed by the App, with only one actor being allowed to respond to a given command (type).

While multi-paradigm frameworks can sometimes lead to an abundance of choice and sprawling implementation, this hybrid model is implemented with a few design preferences:

- Data which needs to be read by many things lives in the ECS world (`Context`). In a pinball context the shorthand here is any data which a display component might need access too lives in Context. Additionally global config, like hardware definitions and mappings are stored in Context.
- Data which is operational -- what the System makes decisions on -- tends to be private to the system, much like an actor. Other systems can mutate this through commands. For example, the `TroughSystem` emits events when the occupancy changes. However a system which manages a physical ball lock may invoke the `BallRemovedFromPlay` command. The trough system has registered itself the handler of that command, and knows upon receiving it to mutate it's internal definition of how many balls are expected to be in the trough, subsequently expecting it's occupancy events.

This approach creates modularity, with a clear interface for display operations, where user-defined systems can interact with framework systems, and framework systems can be entirely replaced with user-defined systems when a custom solution is needed. In fact, Frontbox is modular enough that it could also be an operating system for crane machines or coin pushers (provided it runs FAST hardware) as the mechanics of a pinball game are not hard-coded, but exposed as loadable systems.

#### Abstractions

- `System` contain private state and allow binding of events to behavior
- `Event`s allow Systems to broadcast state change and data to each other
- `Commands` allow fire-and-forget, addressed execution of side effects
- `Context` (ECS world) allows System to share global, mutable state if necessary

#### Context (ECS world)

```rust
// all configured hardware is stored in the Context
let some_switch = ctx.get::<SwitchLookup>().unwrap().get(switches::START_BUTTON);
// systems can also store their own data. Data can be kept read-only through accessors.
```

#### System

```rust
struct Example {
  private_data: u64,
}

impl System for Example {
  fn on_startup(&mut self, ctx: &mut Context) {
    // ...
  }

  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &mut Context) {
    // ...
  }
}
```

#### Commands

```rust
// define a command as a struct
pub struct ExampleCmd(pub arg1: u64, pub arg2: String);

// and register a handler
impl System for Example {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<ExampleCmd>();
  }

  fn on_command(&mut self, command: &dyn Command, ctx: &mut Context) {
    if let Some(cmd) == command.downcast_ref::<ExampleCmd>() {
      // ...
    }
  }
}

// other systems can now use this with
ctx.command(ExampleCommand(arg1, arg2));
```

#### Events

```rust
// define an event as a struct
pub struct ExampleEvent {
  some_data: Vec<u16>
}

// then emit it
ctx.emit(ExampleEvent { some_data: vec![] });
```

#### Event Interrupts

Sometimes there are cases where the normal flow of operation needs to be halted. For example, if a player drains while ball save is active, this would _normally_ emit an event that the player has drained and the turn is over. In these cases it's necessary to allow a system to overide this behavior. This happens by way of event interrupts. Interrupts register themselves with a _priority_, where the App processes interrupts in the highest priority order. This allows, for example, a momentary start-of-ball ball save to take precedence over an extra ball. Event interrupts can be applied to any event within the system.

```rust

fn on_startup(&mut self, ctx: &mut Context) {
  ctx.register_interrupt::<TurnEnd>(100 /* priority */);
}

fn on_interrupt(&mut self, ctx: &mut Context) {
  // for example, a ball save system might do something along the lines of...
  ctx.set_timer(name, Duration::from_secs(5));
}

// ...

fn on_timer(&mut self, name: &'static str, ctx: &mut Context) {
  // once the timer is up, balls can now drain as normal
  ctx.unregister_interrupt::<TurnEnd>();
}
```

#### States

While not a separate feature, states are a pattern to combine `Context` with `is_active`. Storing a globally readable `enum` in `Context` allows other systems to toggle their active state based on it.

```rust
pub enum MachineMode {
  Attract,
  InGame,
  OperatorMenu,
  // ...
}

impl System for Example {
  fn is_active(&self, ctx: &Context) {
    ctx.is(MachineMode::Attract) || ctx.is(OperatorMenu)
  }
}
```

### Libraries

- `fast-protocol` - A Rust native implementation of the FAST Pinball hardware protocol
- `frontbox` - Core framework, Systems, App, routing.
- `frontbox-pinball` - Systems, events, and commands related pinball operation
- `frontbox-turn-based` - Systems, events, and commands realted to the traditional turn-based pinball games (as opposed to head-to-head)

### Example System

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
  fn on_target_hit(&mut self, ctx: &Context) {
    match self.state {
      TargetHitterState::HurryUp => {
        ctx.command(AddPoints(25_000));
        self.on_hurry_up_done();
      }
      TargetHitterState::Building => {
        self.hits = self.hits.saturating_add(1);
        ctx.command(AddPoints(1000));

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
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
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

See [examples](frontbox/examples) for more.
