use frontbox_core::prelude::*;
use frontbox_scene::{Completes, PlayableScene, ScenePlayer, SceneSystemExt};

use crate::ball_management::{AutoPlungerFired, LaunchMethod, PlungeLaneSystem};

/// An autoplunger coil, and optionally the buttons that let the player fire it. Built with `BallManager::standard`.
#[derive(Clone)]
pub struct AutoPlungerConfig {
  pub(super) coil_name: &'static str,
  pub(super) launch_button: Option<SwitchQ>,
  pub(super) launch_ready_scene: Option<PlayableScene>,
  pub(super) before_launch_scene: Option<PlayableScene<Completes>>,
  pub(super) after_launch_scene: Option<PlayableScene<Completes>>,
  /// How long after firing ball management checks that the ball has left the lane switch, before firing again
  pub(super) launch_verification_time: Duration,
}

impl AutoPlungerConfig {
  pub(super) fn new(coil_name: &'static str) -> Self {
    Self {
      coil_name,
      launch_button: None,
      launch_ready_scene: None,
      before_launch_scene: None,
      after_launch_scene: None,
      launch_verification_time: Duration::from_millis(1000),
    }
  }
}

/// Fires the autoplunger coil, either when told to or when a launch button is pressed while armed
pub struct AutoPlungerSystem {
  handle: SystemHandle,
  coil_name: &'static str,
  launch_button: Option<SwitchQ>,
  launch_ready_scene: Option<ScenePlayer>,
  before_launch_scene: Option<ScenePlayer<Completes>>,
  after_launch_scene: Option<PlayableScene<Completes>>,
  armed: bool,
  /// A launch waiting for the before-launch scene to play through, with the cue that ends the wait
  pending_launch: Option<(LaunchMethod, u64)>,
}

impl AutoPlungerSystem {
  pub fn new(config: &AutoPlungerConfig) -> Self {
    Self {
      handle: SystemHandle::default(),
      coil_name: config.coil_name,
      launch_button: config.launch_button.clone(),
      launch_ready_scene: config.launch_ready_scene.clone().map(ScenePlayer::new),
      before_launch_scene: config.before_launch_scene.clone().map(ScenePlayer::new),
      after_launch_scene: config.after_launch_scene.clone(),
      pending_launch: None,
      armed: false,
    }
  }

  pub fn coil_definition(name: &'static str) -> DriverDefinitionBuilder {
    DriverDefinitionBuilder::new(name).mode(
      DriverMode::pulse_kick()
        .initial_pwm_length(
          HardwareValue::config(
            "Autoplunger Touch Time",
            "Duration by which the forks are brought into contact with the ball, before full launch",
            Duration::from_millis(5),
            Ranges::duration(0, 100),
          )
          .key("autoplunger_touch_time"),
        )
        .initial_pwm_power(
          HardwareValue::config(
            "Autoplunger Touch Power",
            "Power used to bring the forks into contact with the ball, before full launch",
            Power::HALF,
            Ranges::full_power(),
          )
          .key("autoplunger_touch_power"),
        )
        .kick_length(
          HardwareValue::config(
            "Autoplunger Coil Launch Time",
            "Duration that the forks exert full power onto the ball (kick)",
            Duration::from_millis(8),
            Ranges::duration(5, 75),
          )
          .key("autoplunger_kick_time"),
        )
        .build(),
    )
  }

  pub fn has_launch_button(&self) -> bool {
    self.launch_button.is_some()
  }

  pub fn is_armed(&self) -> bool {
    self.armed
  }

  /// Launch the ball resting in the plunge lane. With a before-launch scene, the coil fires once the scene has
  /// played through; otherwise it fires now. `AutoPlungerFired` is emitted when the coil fires.
  pub fn launch(&mut self, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    self.begin_launch(LaunchMethod::Auto, ctx);
  }

  /// Call off a launch that's waiting for the before-launch scene, e.g. because the ball has left the lane
  pub fn cancel_launch(&mut self, ctx: &ServiceContext) {
    let Some((_, cue)) = self.pending_launch.take() else {
      return;
    };
    let ctx = &ctx.for_system(self.handle);
    ctx.cues.cancel(cue);
    if let Some(scene) = self.before_launch_scene.as_mut() {
      scene.stop(ctx);
    }
  }

  /// Let a launch button press launch the ball. Does nothing if no launch button is configured.
  pub fn arm_launch_button(&mut self) {
    if self.launch_button.is_none() || self.armed {
      return;
    }
    log::debug!(target: "frontbox::ball_management::autoplunger", "Launch button armed");
    self.armed = true;
  }

  pub fn disarm_launch_button(&mut self, ctx: &ServiceContext) {
    if !self.armed {
      return;
    }
    let ctx = &ctx.for_system(self.handle);
    self.armed = false;
    if let Some(scene) = self.launch_ready_scene.as_mut() {
      scene.stop(ctx);
    }
  }

  /// Play the before-launch scene and then fire, or fire now if there isn't one
  fn begin_launch(&mut self, method: LaunchMethod, ctx: &SystemContext) {
    if self.pending_launch.is_some() {
      return;
    }
    let Some(scene) = self.before_launch_scene.as_mut() else {
      self.fire(method, ctx);
      return;
    };

    scene.restart(ctx);
    let length = scene.scene().length();
    let cue = ctx.cues.start(BeforeLaunchSceneCompleted, length.once());
    self.pending_launch = Some((method, cue));
  }

  fn on_before_launch_scene_completed(&mut self, ctx: &SystemContext) {
    let Some((method, _)) = self.pending_launch.take() else {
      return;
    };
    if let Some(scene) = self.before_launch_scene.as_mut() {
      scene.stop(ctx);
    }
    self.fire(method, ctx);
  }

  fn fire(&mut self, method: LaunchMethod, ctx: &SystemContext) {
    log::info!(target: "frontbox::ball_management::autoplunger", "Firing auto plunger ({:?})", method);
    ctx.drivers.activate(self.coil_name, ActivationMode::Tap);
    ctx.emit(AutoPlungerFired { method });
    if let Some(scene) = &self.after_launch_scene {
      ctx.play_scene(scene);
    }
  }

  fn is_ball_resting(&self, ctx: &SystemContext) -> bool {
    ctx
      .systems.get::<PlungeLaneSystem>()
      .map(|lane| lane.is_ball_resting(ctx.into()))
      .unwrap_or(false)
  }
}

impl System for AutoPlungerSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if self.armed
      && let Some(e) = event.downcast_ref::<SwitchClosed>()
      && let Some(q) = &self.launch_button
      && q.matches(&e.switch)
    {
      // A ball still rolling (e.g. a half plunge) isn't launched; the button stays armed until it settles
      if self.is_ball_resting(ctx) {
        log::info!(target: "frontbox::ball_management::autoplunger", "Launching ball via launch button");
        self.disarm_launch_button(ctx.into());
        self.begin_launch(LaunchMethod::Button, ctx);
      }
    } else if event.is::<BeforeLaunchSceneCompleted>() {
      self.on_before_launch_scene_completed(ctx);
    }
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    // arming has no context to start it with, so it starts here
    if self.armed
      && let Some(scene) = self.launch_ready_scene.as_mut()
    {
      scene.start(ctx);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct BeforeLaunchSceneCompleted;

#[cfg(test)]
mod tests {
  use super::*;

  /// A one second scene on an LED the machine doesn't have
  fn scene() -> PlayableScene<Completes> {
    let scene = frontbox_scene::Scene::load(
      r#"Scene(
        version: 2,
        timing: (duration: 1.0),
        layers: [Leds(leds: Name("launch_led"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))],
      )"#,
    );
    PlayableScene::completing(scene.unwrap()).unwrap()
  }

  fn spawn(config: AutoPlungerConfig) -> (AutoPlungerSystem, TestContext) {
    let ctx = TestContext::default();
    let mut autoplunger = AutoPlungerSystem::new(&config);
    autoplunger.on_spawn(&ctx.sys_ctx());
    (autoplunger, ctx)
  }

  fn fires(ctx: &mut TestContext) -> usize {
    ctx
      .events_emitted()
      .iter()
      .filter(|e| e.event.is::<AutoPlungerFired>())
      .count()
  }

  #[test]
  fn without_a_before_launch_scene_it_fires_at_once() {
    let (mut autoplunger, mut ctx) = spawn(AutoPlungerConfig::new("autoplunger"));
    autoplunger.launch(&ctx.svc_ctx());
    assert_eq!(fires(&mut ctx), 1);
  }

  #[test]
  fn with_a_before_launch_scene_it_fires_once_the_scene_has_played() {
    let mut config = AutoPlungerConfig::new("autoplunger");
    config.before_launch_scene = Some(scene());
    let (mut autoplunger, mut ctx) = spawn(config);

    autoplunger.launch(&ctx.svc_ctx());
    autoplunger.launch(&ctx.svc_ctx()); // already launching
    assert_eq!(fires(&mut ctx), 0);

    autoplunger.on_event(&BeforeLaunchSceneCompleted, &ctx.sys_ctx());
    assert_eq!(fires(&mut ctx), 1);
  }

  #[test]
  fn a_cancelled_launch_doesnt_fire() {
    let mut config = AutoPlungerConfig::new("autoplunger");
    config.before_launch_scene = Some(scene());
    let (mut autoplunger, mut ctx) = spawn(config);

    autoplunger.launch(&ctx.svc_ctx());
    autoplunger.cancel_launch(&ctx.svc_ctx());
    // a cue that was already on its way
    autoplunger.on_event(&BeforeLaunchSceneCompleted, &ctx.sys_ctx());
    assert_eq!(fires(&mut ctx), 0);
  }
}
