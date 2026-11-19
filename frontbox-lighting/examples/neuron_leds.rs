use frontbox_core::prelude::*;
use frontbox_lighting::prelude::*;
use std::io::Write;

/**
 * This example demonstrates declaring light colors, and animating them with scenes
 */

#[derive(Tag)]
pub struct Lane;

pub mod lights {
  use super::*;

  hardware_defs! {
    // like other hardware, lights can be arbitrary tagged for later querying
    pub DEMO1: LedDefinition = LedDefinition::single("demo1").tag(Lane);
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

  let exp_network = ExpNetwork::new(vec![ExpBoard::neuron().wire_led_port(
    1,
    LedPort::ws2812().leds(vec![&lights::DEMO1, &lights::DEMO2, &lights::DEMO3, &lights::DEMO4]),
  )]);

  App::new(BootConfig {
    exp_network,
    ..Default::default()
  })
  .configure(|app| {
    app.system(SceneSystem::new());
    app.system(LayeredLightManager::new());
    app.system(LightExample::new());
  })
  .run()
  .await;
}

/// Fades demo3 from black to purple and back, over and over
const FLASH: &str = r#"Scene(
  version: 2,
  timing: (duration: 0.9, cycle: Forever),
  layers: [Lights(lights: Name("demo3"), surface: Sequence, layer: (fill: (base: Solid(Keyframes(
    keys: [
      (at: 0.0, value: (0, 0, 0, 255), curve: ExponentialInOut),
      (at: 0.45, value: (128, 0, 128, 255), curve: ExponentialInOut),
      (at: 0.9, value: (0, 0, 0, 255), curve: ExponentialInOut),
    ],
    cycle: Once,
  )))))],
)"#;

/// Takes demo4 from black up to red, flashes it yellow, and fades it out, over and over
const SEQUENCE: &str = r#"Scene(
  version: 2,
  timing: (duration: 2.1, cycle: Forever),
  layers: [Lights(lights: Name("demo4"), surface: Sequence, layer: (fill: (base: Solid(Keyframes(
    keys: [
      (at: 0.0, value: (0, 0, 0, 255), curve: EaseInOut),
      (at: 1.5, value: (255, 0, 0, 255), curve: Linear),
      (at: 1.7, value: (255, 255, 0, 255), curve: Linear),
      (at: 2.1, value: (0, 0, 0, 255), curve: Linear),
    ],
    cycle: Once,
  )))))],
)"#;

struct LightExample {
  flash: ScenePlayer,
  sequence: ScenePlayer,
}

impl LightExample {
  fn new() -> Self {
    Self {
      flash: ScenePlayer::new(Scene::load(FLASH).unwrap()),
      sequence: ScenePlayer::new(Scene::load(SEQUENCE).unwrap()),
    }
  }
}

impl System for LightExample {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    // Declare individually
    ctx.declare_lights(&lights::DEMO1.q(), 0, Rgba::blue());
    ctx.declare_lights(&lights::DEMO2.q(), 3, Rgba::yellow().with_alpha(127));

    // Or declare a color per light
    ctx.declare_light_colors(
      &vec![lights::DEMO1.q(), lights::DEMO2.q(), lights::DEMO3.q()],
      5,
      |count| {
        (0..count)
          .map(|i| Rgba::blue().mix_with(Rgba::red(), i as f32 / count as f32))
          .collect()
      },
    );
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    // safe to call every tick: each starts once and plays until the system goes
    self.flash.start(ctx);
    self.sequence.start(ctx);
  }
}
