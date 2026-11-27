use frontbox_core::prelude::*;
use frontbox_scene::prelude::*;

const WARNING_SCENE: &str = "frontbox/128x32/warning.scene";

/// The z warnings are drawn at unless [`DmdWarningSystem::z`] says otherwise: over a game's screens, and under the
/// operator menu
pub const DEFAULT_WARNING_Z: i8 = 100;

/// Shows a warning over whatever else is on the DMD while something is wrong: `frontbox/128x32/warning.scene`, with the
/// warning's message. A warning is raised by one event and cleared by another. It comes with one for the machine's
/// hardware being disconnected ([`MachineDisconnected`] until [`MachineReconnected`]), and a game adds its own with
/// [`warning`](Self::warning). With more than one raised, the last raised is shown.
///
/// A game changes how warnings look by putting its own file under the scene's name in its content folder.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_display::DmdWarningSystem;
/// # #[derive(Event, serde::Serialize, Debug, Clone)] struct DoorJammed;
/// # #[derive(Event, serde::Serialize, Debug, Clone)] struct DoorFreed;
/// # let mut app = App::new(BootConfig { config_path: None, ..Default::default() });
/// app.system(DmdWarningSystem::new().warning::<DoorJammed, DoorFreed>("Door jammed"));
/// ```
pub struct DmdWarningSystem {
  z: i8,
  warnings: Vec<Warning>,
  player: Option<ScenePlayer>,
  /// The `raised` the next warning raised gets
  next_raised: u64,
}

impl DmdWarningSystem {
  pub fn new() -> Self {
    Self {
      z: DEFAULT_WARNING_Z,
      warnings: Vec::new(),
      player: None,
      next_raised: 0,
    }
    .warning::<MachineDisconnected, MachineReconnected>("Hardware disconnected")
  }

  /// Shows `message` from when a `Raised` is emitted until a `Cleared` is
  pub fn warning<Raised: Event, Cleared: Event>(mut self, message: &'static str) -> Self {
    self.warnings.push(Warning {
      message,
      raised_by: is::<Raised>,
      cleared_by: is::<Cleared>,
      raised: None,
    });
    self
  }

  /// Draws warnings at `z` among everything on the display. [`DEFAULT_WARNING_Z`] unless this says otherwise.
  pub fn z(mut self, z: i8) -> Self {
    self.z = z;
    self
  }

  /// The message to show: that of the warning raised last
  fn message(&self) -> Option<&'static str> {
    self
      .warnings
      .iter()
      .filter_map(|warning| Some((warning.raised?, warning.message)))
      .max()
      .map(|(_, message)| message)
  }

  /// Raises and clears the warnings `event` is for. Returns whether any changed.
  fn update(&mut self, event: &dyn Event) -> bool {
    let mut changed = false;
    for warning in &mut self.warnings {
      if warning.raised.is_none() && (warning.raised_by)(event) {
        log::warn!(target: "frontbox::display", "⚠️ Warning raised: {}", warning.message);
        self.next_raised += 1;
        warning.raised = Some(self.next_raised);
        changed = true;
      } else if warning.raised.is_some() && (warning.cleared_by)(event) {
        log::info!(target: "frontbox::display", "Warning cleared: {}", warning.message);
        warning.raised = None;
        changed = true;
      }
    }
    changed
  }
}

impl Default for DmdWarningSystem {
  fn default() -> Self {
    Self::new()
  }
}

impl System for DmdWarningSystem {
  fn description() -> Option<&'static str> {
    Some(
      "Shows a warning on the DMD while something is wrong, such as the hardware being disconnected",
    )
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    match Scene::open(&ctx.resources, WARNING_SCENE) {
      Ok(mut scene) => {
        WARNING_SENDS.log_mismatches(WARNING_SCENE, &scene);
        scene
          .layers
          .iter_mut()
          .filter_map(SceneLayer::as_display_mut)
          .for_each(|layer| layer.z = self.z);
        self.player = Some(ScenePlayer::new(PlayableScene::new(scene)));
      }
      Err(error) => log::error!(
        target: "frontbox::display",
        "Warnings can't be drawn: {error}{}",
        crate::bundle_hint(WARNING_SCENE)
      ),
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if !self.update(event) {
      return;
    }
    let message = self.message();
    let Some(player) = &mut self.player else {
      return;
    };
    match message {
      Some(message) => {
        player.set_variable("message", message, ctx);
        player.start(ctx);
      }
      None => player.stop(ctx),
    }
  }
}

/// Something wrong that's shown from one event until another
struct Warning {
  message: &'static str,
  raised_by: fn(&dyn Event) -> bool,
  cleared_by: fn(&dyn Event) -> bool,
  /// When it was raised, among the others. `None` while it isn't.
  raised: Option<u64>,
}

fn is<E: Event>(event: &dyn Event) -> bool {
  event.is::<E>()
}

const WARNING_SENDS: Sends = Sends {
  variables: &[SentVariable {
    name: "message",
    sent_as: VariableType::Text,
    values: &[],
    about: "What's wrong",
  }],
};

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Event, serde::Serialize, Debug, Clone)]
  struct Jammed;
  #[derive(Event, serde::Serialize, Debug, Clone)]
  struct Freed;

  fn system() -> DmdWarningSystem {
    DmdWarningSystem::new().warning::<Jammed, Freed>("Jammed")
  }

  #[test]
  fn a_warning_shows_from_its_first_event_until_its_second() {
    let mut system = system();
    assert_eq!(system.message(), None);
    assert!(system.update(&MachineDisconnected));
    assert_eq!(system.message(), Some("Hardware disconnected"));
    assert!(!system.update(&MachineDisconnected), "it's already raised");
    assert!(!system.update(&Freed), "that clears another warning");
    assert!(system.update(&MachineReconnected));
    assert_eq!(system.message(), None);
  }

  #[test]
  fn the_warning_raised_last_is_shown_until_its_cleared() {
    let mut system = system();
    system.update(&Jammed);
    system.update(&MachineDisconnected);
    assert_eq!(system.message(), Some("Hardware disconnected"));
    system.update(&MachineReconnected);
    assert_eq!(system.message(), Some("Jammed"));
  }

  #[test]
  fn a_raised_warning_plays_the_scene_and_a_cleared_one_stops_it() {
    let mut context = TestContext::default();
    context.set_resources(crate::bundle::bundle());
    context.insert_system(SceneSystem::new());
    let ctx = context.sys_ctx();
    let mut system = system();
    system.on_spawn(&ctx);
    let shown = || ctx.systems.get::<SceneSystem>().unwrap().shown().count();

    system.on_event(&MachineDisconnected, &ctx);
    assert_eq!(shown(), 1);
    system.on_event(&MachineReconnected, &ctx);
    assert_eq!(shown(), 0);
  }

  #[test]
  fn the_bundled_scene_fits_what_is_sent() {
    let scene = Scene::open(&crate::bundle::bundle(), WARNING_SCENE).unwrap();
    assert_eq!(WARNING_SENDS.check(&scene), []);
  }
}
