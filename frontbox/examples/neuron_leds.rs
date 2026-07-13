use frontbox::animation::*;
use frontbox::prelude::*;
use std::io::Write;

/**
 * This example demonstrates how to use the animation system to various LED effects
 */

pub mod leds {
  use super::*;

  hardware_defs! {
    // like other hardware, LEDs can be arbitrary tagged for later querying
    pub DEMO1: LedDefinition = LedDefinition::single("demo1").tag(tags::Lane);
    pub DEMO2: LedDefinition = LedDefinition::single("demo2");
    pub DEMO3: LedDefinition = LedDefinition::single("demo3");
    pub DEMO4: LedDefinition = LedDefinition::single("demo4");
  }
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let exp_network = vec![ExpansionBoard::neuron().wire_led_port(
    0,
    LedPort::ws2812().leds(vec![&leds::DEMO1, &leds::DEMO2, &leds::DEMO3, &leds::DEMO4]),
  )];

  App::boot(
    "/dev/ttyACM0",
    "/dev/ttyACM1",
    IoNetwork::empty(),
    exp_network,
  )
  .await
  .configure(|app| {
    app.system(LedSystem::new());
    app.system(LedExample::new());
  })
  .run()
  .await;
}

struct LedExample {
  flash: Box<dyn Animation<Duration, Rgba<u8>>>,
  seq: Box<dyn Animation<Duration, Rgba<u8>>>,
}

impl LedExample {
  fn new() -> Self {
    Self {
      flash: Tween::boxed(
        Duration::from_millis(450),
        Curve::ExponentialInOut,
        vec![Rgba::black(), Rgba::purple()],
        AnimationCycle::Forever,
      ),
      seq: Sequence::boxed(
        vec![
          Tween::boxed(
            Duration::from_millis(1500),
            Curve::QuadraticInOut,
            vec![Rgba::black(), Rgba::red()],
            AnimationCycle::Once,
          ),
          Tween::boxed(
            Duration::from_millis(200),
            Curve::Sinusoid,
            vec![Rgba::red(), Rgba::yellow()],
            AnimationCycle::Once,
          ),
          Tween::boxed(
            Duration::from_millis(400),
            Curve::Linear,
            vec![Rgba::yellow(), Rgba::black()],
            AnimationCycle::Once,
          ),
        ],
        AnimationCycle::Forever,
      ),
    }
  }
}

impl System for LedExample {
  fn on_spawn(&mut self, ctx: &Context) {
    // Declare individually
    ctx.declare_leds(&leds::DEMO1, Rgba::blue());
    ctx.declare_leds(&leds::DEMO2, Rgba::yellow().with_alpha(127));

    // Or declare as a sequence
    ctx.declare_leds(
      vec![&leds::DEMO1, &leds::DEMO2, &leds::DEMO3],
      Colors::gradient(vec![Rgba::blue(), Rgba::red()]).reverse(),
    );
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context) {
    // tick animations to update their internal state
    self.flash.accumulate(delta);
    self.seq.accumulate(delta);

    // re-declare LEDs with updated animated colors
    ctx.declare_leds(&leds::DEMO3, self.flash.sample());
    ctx.declare_leds(&leds::DEMO4, self.seq.sample());
  }
}
