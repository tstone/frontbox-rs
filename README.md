# Frontbox

A Rust native framework for interacting with FAST pinball hardware, built for efficiency and modularity.

> [!WARNING]
> Pre-alpha work in progress

### Preview (Subject to Change)

**Frontbox** is built around the unit of a `System`. Systems receive events and enqueue commands. Systems are built on Rust structs, and can thus maintain their own state. 

#### Example System
This system implements a basic pinball "mode". A target is illuminated and must be struck 3 times. Each hit grants 1000 points. After 3 hits, the target will begin flashing. The player has 20 seconds to hit it again for 10,000 points (hurry up shot). After 20 seconds or being hit a 4th time the mode resets.

- `SwitchClosed` event monitors the target's switch
- `ctx.set_timer` and `TimerComplete` event monitors the hurry up timer
- `self.hurry_up_active` and `self.hits` manage state
- `fn leds` sets the LED state for the framework to apply (declarative)

```rust
const HURRY_UP_TIMER: &'static str = "hurry_up";

#[derive(Debug, Clone)]
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

  fn on_target_hit(&mut self, ctx: &Context) {
    if self.hurry_up_active {
      ctx.add_points(10000);
      ctx.add_bonus(1000);
      self.reset();
    } else {
      self.hits = self.hits.saturating_add(1);
      self.add_points(1000);
    
      if self.hits == 3 {
        self.hurry_up_active = true;
        ctx.set_timer(HURRY_UP_TIMER, Duration::from_secs(20), TimerMode::Once);
      }
    }
  }

  fn on_hurry_up_done(&mut self) {
    self.reset();
  }
}

impl System for TargetHitter {
  fn on_startup(&mut self, ctx: &Context) {
    ctx.subscribe::<SwitchClosed>(|event, ctx| {
      if event.switch.id == self.target_switch_id {
        self.on_target_hit(ctx);
      }
    });

    ctx.subscribe::<TimerComplete>(|event, _ctx| {
      if event.name == HURRY_UP_TIMER {
        self.on_hurry_up_done();
      }
    });
  }

  fn leds(&mut self, delta_time: Duration) -> LedStates {
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

See [examples](/tree/main/frontbox/examples) for more.
