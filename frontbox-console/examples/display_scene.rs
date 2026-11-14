//! Plays a scene on a 128 x 32 PIN2DMD, and in the console at <http://localhost:3000>.
//!
//! ```sh
//! cargo run -p frontbox-console --example display_scene -- [scene] [io port] [exp port]
//! ```
//!
//! The scene is named by its path in `examples/content`, and is `display-test.scene` if not given. With a Neuron's ports
//! it runs on the machine, with no hardware defined, and opens the PIN2DMD. Without them it runs virtually, where the
//! display is only shown in the console.

use std::io::Write;

use frontbox_console::WebTracer;
use frontbox_core::prelude::*;
use frontbox_display::*;
use frontbox_dmd::*;
use frontbox_gfx::scene::Content;
use frontbox_scene::*;

#[tokio::main]
async fn main() {
  env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  let mut args = std::env::args().skip(1);
  let name = args.next().unwrap_or_else(|| "display-test.scene".into());
  let platform = match (args.next(), args.next()) {
    (Some(io), Some(exp)) => Platform::Neuron {
      io_net_port_path: io.leak(),
      exp_port_path: exp.leak(),
      watchdog_interval: Duration::from_millis(1500),
    },
    _ => Platform::Virtual,
  };

  let content = Content::new(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/content"));
  let scene = load(&content, &name).unwrap_or_else(|err| panic!("Couldn't play {name}: {err}"));

  App::new(BootConfig {
    platform,
    config_path: None,
    metrics_path: None,
    content_path: Some(content.root().into()),
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
    app.system(PlayScene { scene, player: None });
    app.tracer(console);
  })
  .run()
  .await;
}

fn load(content: &Content, name: &str) -> Result<PlayableScene, String> {
  let path = content.scene_path(name).map_err(|err| err.to_string())?;
  let source = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
  let scene = Scene::load(&source).map_err(|err| err.to_string())?;
  Ok(PlayableScene::new(scene.with_images(content).map_err(|err| err.to_string())?))
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
