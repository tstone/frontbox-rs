use frontbox_core::prelude::*;
use frontbox_gfx::layout::Line;
use frontbox_gfx::scene::{Frame, Params, Scene, Value};

use crate::*;

/// Plays a [`frontbox_gfx::scene::Scene`] on a list of LEDs, laid out as a sequence
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_led::prelude::*;
/// # use frontbox_gfx::scene::*;
/// let scene = file::load(r#"(
///   version: 1,
///   layers: [(paint: (kind: Solid(Param("color"))))],
/// )"#).unwrap();
///
/// let mut program = LedScene::new(LedQ::every(), scene);
/// program.set_param("color", Value::Color(Color(Rgba::red())));
/// ```
#[derive(Clone)]
pub struct LedScene {
  ids: Box<dyn Contextual<LedIdentifications> + Send + Sync>,
  scene: Scene,
  params: Params,
  elapsed: Duration,
  playing: bool,
}

impl LedScene {
  pub fn new<T: Contextual<LedIdentifications> + Send + Sync + 'static>(targets: T, scene: Scene) -> Self {
    Self {
      ids: Box::new(targets),
      scene,
      params: Params::new(),
      elapsed: Duration::ZERO,
      playing: true,
    }
  }

  /// Sets a parameter, taking the place of the scene's default
  pub fn set_param(&mut self, name: impl Into<String>, value: Value) {
    self.params.insert(name.into(), value);
  }

  /// Move time forward and declare the LEDs' colors
  pub fn apply(&mut self, delta: Duration, ctx: &SystemContext) {
    if !self.playing {
      return;
    }
    self.elapsed += delta;

    ctx.declare_led_colors(&self.ids, |count| {
      let line = Line::new(count);
      let frame = Frame {
        time: self.elapsed.as_secs_f32(),
        stage: line.stage(),
        params: &self.params,
      };
      line.samples().map(|s| self.scene.sample(&s, &frame)).collect()
    });
  }

  /// Play from where the scene is
  pub fn play(&mut self) {
    self.playing = true;
  }

  /// Stop playing and clear the LEDs
  pub fn stop(&mut self, ctx: &SystemContext) {
    self.playing = false;
    ctx.undeclare_leds(&self.ids);
  }

  /// Back to the start of the scene
  pub fn reset(&mut self) {
    self.elapsed = Duration::ZERO;
  }

  pub fn is_playing(&self) -> bool {
    self.playing
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::{Curve, Cycle};
  use frontbox_gfx::scene::*;
  use frontbox_gfx::prelude::*;

  use super::*;

  fn three_leds() -> (TestContext, Vec<LedAddress>) {
    let mut context = TestContext::default();
    context.insert_system(LedSystem::new());
    let addresses: Vec<_> = (1..=3).map(|i| LedAddress::new(ExpAddress::default(), i)).collect();
    for (i, address) in addresses.iter().enumerate() {
      context.insert_led(LED {
        name: format!("led{}", i + 1),
        address: address.clone(),
        ..Default::default()
      });
    }
    (context, addresses)
  }

  fn colors(context: &TestContext, addresses: &[LedAddress]) -> Vec<Option<Rgba<u8>>> {
    let ctx = context.sys_ctx();
    let led_system = ctx.systems.expect::<LedSystem>();
    addresses
      .iter()
      .map(|a| led_system.declarations_for(a).first().map(|d| d.color))
      .collect()
  }

  fn scene(paint: PaintNode) -> Scene {
    Scene::new(vec![Layer::new(Paint::new(paint))])
  }

  #[test]
  fn gradient_runs_along_the_leds() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(
      LedQ::names(vec!["led1", "led2", "led3"]),
      scene(PaintNode::Gradient {
        stops: vec![Stop::new(0.0, Color(Rgba::red())), Stop::new(1.0, Color(Rgba::blue()))],
        space: ColorSpace::Rgb,
      }),
    );

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(
      colors(&context, &addresses),
      vec![Some(Rgba::red()), Some(Rgba([127, 0, 128, 255])), Some(Rgba::blue())]
    );
  }

  #[test]
  fn time_moves_tracks() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(
      LedQ::name("led1"),
      scene(PaintNode::Solid(Prop::Track(Track {
        keys: vec![
          Key::new(0.0, Color(Rgba([0, 0, 0, 255])), Curve::Linear),
          Key::new(1.0, Color(Rgba([200, 0, 0, 255])), Curve::Linear),
        ],
        cycle: Cycle::Once,
      }))),
    );

    program.apply(Duration::from_millis(500), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba([100, 0, 0, 255])));
  }

  #[test]
  fn stop_clears_the_leds() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(LedQ::name("led1"), scene(PaintNode::Solid(Color(Rgba::red()).into())));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::red()));

    program.stop(&context.sys_ctx());
    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], None);
  }

  #[test]
  fn params_set_by_the_game() {
    let (context, addresses) = three_leds();
    let mut program = LedScene::new(
      LedQ::name("led1"),
      scene(PaintNode::Solid(Prop::Param("color".into()))),
    );
    program.set_param("color", Value::Color(Color(Rgba::green())));

    program.apply(Duration::from_millis(16), &context.sys_ctx());
    assert_eq!(colors(&context, &addresses)[0], Some(Rgba::green()));
  }
}
