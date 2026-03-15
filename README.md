# Frontbox

Frontbox is a Rust native, "ECS light" framework for running pinball machines, built on FAST pinball hardware.

> [!WARNING]
> Pre-alpha work in progress

### Overview

**Frontbox** is built around the unit of a `System`. Systems receive events, enqueue commands, manage their own state, and mutably interact with the ECS "world" (called `Context` in Frontbox).

#### Features

- Implementation of modern FAST protocol
- Extensible events system
- Flexible, hierarchical isolation of concerns (`System`)
- Player and Co-op/team support
- LED animation framework

Demo on prototype hardware: https://www.youtube.com/shorts/GHNZA3x88v8

#### Vs ECS

- Whereas many ECS frameworks eliminate or minimize systems, Frontbox is built around systems as a first class citizen.
- Whereas many ECS frameworks make systems stateless, Frontbox systems are built on Rust structs making them stateful (more like an Actor than a function).
- Whereas ECS tends to emphasize components for data, Frontbox only allows singleton data to be stored ("Resources").
- Whereas ECS tends to run systems on a rendering tick, Frontbox runs systems on events.

### Architecture

#### Data

```rust
// all configured hardware is stored in the world (Context)
let some_switch = ctx.get::<SwitchLookup>().unwrap().get(switches::START_BUTTON);

```

#### System

```rust
struct Example;

impl System for Example {
  fn on_startup(&mut self, ctx: &mut Context) {
    // ...
  }

  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &mut Context) {
    // ...
  }
}
```

#### Tools

- `System` contain private state and allow binding of events to behavior
- `Event`s allow Systems to broadcast state change and data to each other
- `Commands` allow Systems to define addressed, exactly once handling of events
- `Context` (ECS world) allows System to share global, mutable state if necessary

#### Events vs Context

- Prefer Events and private state
- Use Context when display functions need to access state
- Use Context when other systems need "reference" data

### Example System

This system implements a basic pinball "mode". A target is illuminated and must be struck 3 times. Each hit grants 1000 points. After 3 hits, the target will begin flashing. The player has 20 seconds to hit it again for 10,000 points (hurry up shot). After 20 seconds or being hit a 4th time the mode resets.

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
  hurry_up_active: bool,
  // ids for target switch and LED indicator
  target_switch_id: &'static str,
  indicator_id: &'static str,
}

impl TargetHitter {
  pub fn new(target_switch_id: &'static str, indicator_id: &'static str) -> Box<Self> {
    Box::new(Self {
      target_switch_id,
      indicator_id,
      hits: 0,
      hurry_up_active: false,
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

  fn on_target_hit(&mut self, ctx: &Context, cmds: &mut Commands) {
    if self.hurry_up_active {
      cmds.add_points(10000);
      cmds.add_bonus(1000);
      self.on_hurry_up_done();
    } else {
      self.hits = self.hits.saturating_add(1);
      self.add_points(1000);

      if self.hits == 3 {
        self.hurry_up_active = true;
        cmds.set_timer(HURRY_UP_TIMER, Duration::from_secs(20), TimerMode::Once);
      }
    }
  }

  fn on_hurry_up_done(&mut self) {
    self.reset();
  }
}

impl System for TargetHitter {
  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &Context, cmds: &mut Commands) {
    handle_event!(event, {
      SwitchClosed => |e| {
        if event.switch.id == self.target_switch_id {
          self.on_target_hit(ctx, cmds);
        }
      }
    })
  }

  fn on_timer(&mut self, name: &'static str, _ctx: &Context, _cmds: &mut Commands) {
    if event.name == HURRY_UP_TIMER {
      self.on_hurry_up_done();
    }
  }

  fn leds(&mut self, delta_time: Duration, _ctx: &Context) -> LedStates {
    // show the flashing state if hurry up is active otherwise use a static color
    if self.hurry_up_active {
      LedDeclarationBuilder::new(delta_time)
        .next_frame(self.flash_anim)
        .collect()
    } else {
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
```

See [examples](frontbox/examples) for more.
