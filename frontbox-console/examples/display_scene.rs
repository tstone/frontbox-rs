//! Plays a scene on a 128 x 32 PIN2DMD, and in the console at <http://localhost:3000>.
//!
//! ```sh
//! cargo run -p frontbox-console --example display_scene -- [--content <folder>] [scene] [io port] [exp port]
//! ```
//!
//! The scene is named by its path in the content folder, which is `examples/content` unless `--content` names another,
//! and is `display-test.scene` if not given. The scenes frontbox comes with can be named too, such as
//! `frontbox/128x32/points.scene`. The console's scene editor saves to that folder, so
//! `--content frontbox-display/assets/scenes` edits the scenes frontbox comes with. With a Neuron's ports it runs on the
//! machine, with no hardware defined, and opens the PIN2DMD. Without them it runs virtually, where the display is only
//! shown in the console.

use std::io::Write;

use frontbox_console::WebTracer;
use frontbox_core::prelude::*;
use frontbox_display::*;
use frontbox_dmd::*;
use frontbox_scene::*;

#[tokio::main]
async fn main() {
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let mut args: Vec<String> = std::env::args().skip(1).collect();
  let folder = match args.iter().position(|arg| arg == "--content") {
    Some(at) if at + 1 < args.len() => args.drain(at..at + 2).nth(1).unwrap(),
    Some(_) => panic!("--content needs a folder"),
    None => concat!(env!("CARGO_MANIFEST_DIR"), "/examples/content").into(),
  };
  let mut args = args.into_iter();
  let name = args.next().unwrap_or_else(|| "display-test.scene".into());
  let platform = match (args.next(), args.next()) {
    (Some(io), Some(exp)) => Platform::Neuron {
      io_net_port_path: io.leak(),
      exp_port_path: exp.leak(),
      watchdog_interval: Duration::from_millis(1500),
    },
    _ => Platform::Virtual,
  };

  let content = content_with_bundled(Some(folder.into()));
  let scene = Scene::open(&content, &name).unwrap_or_else(|err| panic!("Couldn't play {name}: {err}"));

  App::new(BootConfig {
    platform,
    config_path: None,
    metrics_path: None,
    content_path: content.root().map(Into::into),
    // a display can show far more than LEDs, so it's worth more frames
    system_interval: Duration::from_millis(33),
    ..Default::default()
  })
  .configure(|app| {
    let console = WebTracer::new();
    app.system(SceneSystem::new());
    app.system(DisplaySystem::new().display(
      DisplayDefinition::new("Dmd", Pin2Dmd::new(128, 32, PanelType::Rgb)).mirror(console.display_view()),
    ));
    app.system(PlayScene { scene: PlayableScene::new(scene), player: None });
    app.tracer(console);
  })
  .run()
  .await;
}

/// Plays the scene for as long as the app runs
struct PlayScene {
  scene: PlayableScene,
  player: Option<ScenePlayer>,
}

impl System for PlayScene {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.player = Some(ScenePlayer::started(self.scene.clone(), ctx));
  }
}
