# Frontbox

A Rust native framework for interacting with FAST pinball hardware, built for efficiency and modularity.

> [!WARNING]
> Pre-alpha work in progress

### Preview (Subject to Change)

**Frontbox** is built around the unit of a `System`. Systems receive events and enqueue commands. Systems are built on Rust structs, and can thus maintain their own state. For example, here's a simple system that does the following:

- Requires the player to hit the same target 3 times (LED lit yellow, orange, red)
- Once this is achieved a timer starts to hit the target a 4th time for bonus (LED red flashing)

```rust
const HURRY_UP_TIMER: &'static str = "hurry_up";

#[derive(Debug, Clone)]
struct TargetHitter {
  // current times this target has been hit
  hits: u8,
  // animation for bonus hit
  flash_anim: Box<dyn Animation<Color>>,
  hurry_up_acative: bool,
  // ids for target switch and LED indicator
  target_switch_id: &'static str,
  indicator_id: &'static str,
}

impl TargetHitter {
  pub fn new(target_switch_id: &'static str, indicator_id: &'static str) -> Box<Self> {
    Box::new(Self {
      target_switch_id,
      indicator_id,
      required_hits,
      hits: 0,
      hurry_up_acative: false,
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
    self.hits = self.hits.saturating_add(1);      

    if self.hits == 3 {
      self.hurry_up_active = true;
      ctx.set_timer(HURRY_UP_TIMER, Duration::from_secs(1), TimerMode::Once);
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
