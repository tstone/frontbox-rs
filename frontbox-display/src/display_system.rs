use std::collections::HashSet;
use std::sync::mpsc;

use frontbox_core::prelude::*;
use frontbox_gfx::layout::Grid;
use frontbox_gfx::prelude::*;
use frontbox_gfx::scene::{Sample, Surface};
use frontbox_scene::{DisplayLayer, Playback, SceneLayer, SceneSystem};
use image::Rgb;

use crate::display_manager::DisplayManager;
use crate::{DisplayDefinition, DisplayMirror, DisplaySize};

/// Draws the `Display` layers of the scenes the [`SceneSystem`] is playing, on the displays added to it. Each display
/// shows the layers that name it, by `z` and then in the order their playbacks started, and is sent a frame only when
/// it changes.
///
/// ```rust,ignore
/// app.system(DisplaySystem::new().display(DisplayDefinition::new("Dmd", Pin2Dmd::new(128, 32, PanelType::Rgb))));
/// ```
pub struct DisplaySystem {
  displays: Vec<Display>,
  /// Displays that scenes have layers for but weren't added, so each is only warned about once
  unknown: HashSet<String>,
}

impl DisplaySystem {
  pub fn new() -> Self {
    Self { displays: Vec::new(), unknown: HashSet::new() }
  }

  /// Adds a display. A second display with the same name takes the place of the first.
  pub fn display(mut self, definition: DisplayDefinition) -> Self {
    if self.displays.iter().any(|display| display.name == definition.name) {
      log::warn!(target: "frontbox::display", "🖥️ {} was added twice; the last one wins", definition.name);
      self.displays.retain(|display| display.name != definition.name);
    }
    self.displays.push(Display::new(definition));
    self
  }

  /// Each display's layers among the playbacks the `SceneSystem` shows, in the order they're drawn
  fn layers<'a>(&mut self, scenes: &'a SceneSystem) -> Vec<Vec<(&'a Playback, &'a DisplayLayer)>> {
    let mut layers: Vec<Vec<_>> = self.displays.iter().map(|_| Vec::new()).collect();
    let all = scenes.shown().flat_map(|(_, playback)| {
      playback.scene().layers.iter().filter_map(SceneLayer::as_display).map(move |layer| (playback, layer))
    });
    for (playback, layer) in all {
      match self.displays.iter().position(|display| display.name == layer.display) {
        Some(index) => layers[index].push((playback, layer)),
        None => {
          if self.unknown.insert(layer.display.clone()) {
            log::warn!(target: "frontbox::display", "🖥️ A scene has a layer for {}, which isn't a display that was added", layer.display);
          }
        }
      }
    }
    // stable, so on the same z, playbacks stay in the order they started and layers in the order their scene has them
    layers.iter_mut().for_each(|layers| layers.sort_by_key(|(_, layer)| layer.z));
    layers
  }
}

impl Default for DisplaySystem {
  fn default() -> Self {
    Self::new()
  }
}

impl System for DisplaySystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    for display in &mut self.displays {
      display.mirrors.iter_mut().for_each(|mirror| mirror.attach(&display.name, display.size));
      let Some(manager) = display.manager.take() else {
        continue;
      };
      if ctx.platform == PlatformKind::Virtual {
        log::info!(target: "frontbox::display", "🖥️ On a virtual platform, so {} isn't opened", display.name);
        continue;
      }
      if let Err(e) = std::thread::Builder::new()
        .name(format!("display {}", display.name))
        .spawn(move || manager.run())
      {
        log::error!(target: "frontbox::display", "🖥️ Couldn't start the thread for {}: {}", display.name, e);
      }
    }
  }

  fn on_despawn(&mut self, _ctx: &SystemContext) {
    for display in &mut self.displays {
      display.show(vec![Rgb([0, 0, 0]); display.size.pixels()]);
    }
  }

  fn on_render(&mut self, ctx: &SystemContext) {
    let Some(scenes) = ctx.systems.get::<SceneSystem>() else {
      return;
    };
    let layers = self.layers(&scenes);
    for (display, layers) in self.displays.iter_mut().zip(layers) {
      let frame = display.draw(&layers);
      display.show(frame);
    }
  }
}

/// A display added to the [`DisplaySystem`], and what it last showed
struct Display {
  name: String,
  size: DisplaySize,
  /// Where each pixel is sampled, row-major from the top left
  samples: Vec<Sample>,
  /// Drawn into each frame, so a frame's layers don't need a buffer of their own
  pixels: Vec<Rgba<u8>>,
  /// The last frame sent, so an unchanged one isn't sent again. Empty until the first.
  last_frame: Vec<Rgb<u8>>,
  device: mpsc::Sender<Vec<Rgb<u8>>>,
  /// Until the system spawns and starts it on its own thread
  manager: Option<DisplayManager>,
  mirrors: Vec<Box<dyn DisplayMirror>>,
}

impl Display {
  fn new(definition: DisplayDefinition) -> Self {
    let DisplayDefinition { name, size, device, hard_edges, mirrors } = definition;
    let (width, height) = (size.width as usize, size.height as usize);
    let grid = Grid::new(width, height, Vec2::new(width as f32, height as f32));
    // a sample with no radius is either in a shape or out of it
    let samples = grid.samples().map(|at| if hard_edges { Sample::new(at.pos, 0.0) } else { at }).collect();
    let (tx, rx) = mpsc::channel();
    Self {
      manager: Some(DisplayManager::new(name.clone(), device, rx)),
      name,
      size,
      samples,
      pixels: vec![Rgba([0, 0, 0, 0]); size.pixels()],
      last_frame: Vec::new(),
      device: tx,
      mirrors,
    }
  }

  /// `layers` over black, bottom first
  fn draw(&mut self, layers: &[(&Playback, &DisplayLayer)]) -> Vec<Rgb<u8>> {
    self.pixels.fill(Rgba([0, 0, 0, 0]));
    let size = Vec2::new(self.size.width as f32, self.size.height as f32);
    let radius = self.samples.first().map_or(0.0, |at| at.radius);
    for (playback, layer) in layers {
      let moment = playback.moment();
      let frame = playback.scene().frame(&moment, Surface::Area, size, None);
      let prepared = layer.layer.prepare(&frame, radius);
      for (pixel, at) in self.pixels.iter_mut().zip(&self.samples) {
        *pixel = prepared.sample(at).composite_over(*pixel);
      }
    }
    let over_black = self.pixels.iter().map(|pixel| pixel.composite_over(Rgba::black()));
    over_black.map(|Rgba([r, g, b, _])| Rgb([r, g, b])).collect()
  }

  /// Sends `frame` to the device and mirrors, unless it's the one they already have
  fn show(&mut self, frame: Vec<Rgb<u8>>) {
    if frame == self.last_frame {
      return;
    }
    self.mirrors.iter_mut().for_each(|mirror| mirror.show(&frame));
    // fails once the device's thread has gone, or on a virtual platform where it never started
    let _ = self.device.send(frame.clone());
    self.last_frame = frame;
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use frontbox_scene::{PlayableScene, Scene};

  use crate::DisplayDevice;

  use super::*;

  /// 4 x 2, so a frame is short enough to write out
  const SIZE: DisplaySize = DisplaySize { width: 4, height: 2 };
  const RED: Rgb<u8> = Rgb([255, 0, 0]);
  const BLUE: Rgb<u8> = Rgb([0, 0, 255]);
  const BLACK: Rgb<u8> = Rgb([0, 0, 0]);

  struct Device;

  impl DisplayDevice for Device {
    fn size(&self) -> DisplaySize {
      SIZE
    }

    fn open(&mut self) -> Result<(), String> {
      Ok(())
    }

    fn write(&mut self, _frame: &[Rgb<u8>]) -> Result<(), String> {
      Ok(())
    }
  }

  /// Keeps what it's given, for the test to read
  #[derive(Clone, Default)]
  struct Recorder {
    attached: Arc<Mutex<Option<(String, DisplaySize)>>>,
    frames: Arc<Mutex<Vec<Vec<Rgb<u8>>>>>,
  }

  impl Recorder {
    fn frames(&self) -> Vec<Vec<Rgb<u8>>> {
      self.frames.lock().unwrap().clone()
    }
  }

  impl DisplayMirror for Recorder {
    fn attach(&mut self, display: &str, size: DisplaySize) {
      *self.attached.lock().unwrap() = Some((display.into(), size));
    }

    fn show(&mut self, frame: &[Rgb<u8>]) {
      self.frames.lock().unwrap().push(frame.to_vec());
    }
  }

  fn context(definition: DisplayDefinition) -> TestContext {
    let mut context = TestContext::default();
    context.insert_system(SceneSystem::new());
    context.insert_system(DisplaySystem::new().display(definition));
    context.sys_ctx().systems.with::<DisplaySystem, _>(|displays| displays.on_spawn(&context.sys_ctx()));
    context
  }

  fn recorded() -> (TestContext, Recorder) {
    let recorder = Recorder::default();
    (context(DisplayDefinition::new("Dmd", Device).mirror(recorder.clone())), recorder)
  }

  fn play(context: &TestContext, layers: &str) {
    let scene = Scene::load(&format!("Scene(version: 2, layers: [{layers}])")).unwrap();
    context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| scenes.start(1, &PlayableScene::new(scene)));
  }

  fn render(context: &TestContext) {
    context.sys_ctx().systems.with::<DisplaySystem, _>(|displays| displays.on_render(&context.sys_ctx()));
  }

  fn solid(display: &str, z: i8, (r, g, b): (u8, u8, u8)) -> String {
    format!(r#"Display(display: "{display}", z: {z}, layer: (fill: (base: Solid(Fixed({r}, {g}, {b}, 255)))))"#)
  }

  #[test]
  fn a_mirror_is_told_which_display_it_shows() {
    let (_context, recorder) = recorded();
    assert_eq!(*recorder.attached.lock().unwrap(), Some(("Dmd".into(), SIZE)));
  }

  #[test]
  fn a_layer_is_drawn_on_the_display_it_names() {
    let (context, recorder) = recorded();
    play(&context, &format!("{}, {}", solid("Dmd", 0, (255, 0, 0)), solid("Other", 1, (0, 0, 255))));
    render(&context);
    assert_eq!(recorder.frames(), [vec![RED; 8]]);
  }

  #[test]
  fn nothing_playing_is_black() {
    let (context, recorder) = recorded();
    render(&context);
    assert_eq!(recorder.frames(), [vec![BLACK; 8]]);
  }

  #[test]
  fn layers_are_drawn_by_z_across_playbacks() {
    let (context, recorder) = recorded();
    play(&context, &solid("Dmd", 1, (255, 0, 0)));
    play(&context, &solid("Dmd", 0, (0, 0, 255)));
    render(&context);
    assert_eq!(recorder.frames(), [vec![RED; 8]]);
  }

  #[test]
  fn on_the_same_z_the_later_playback_is_on_top() {
    let (context, recorder) = recorded();
    play(&context, &solid("Dmd", 0, (255, 0, 0)));
    play(&context, &solid("Dmd", 0, (0, 0, 255)));
    render(&context);
    assert_eq!(recorder.frames(), [vec![BLUE; 8]]);
  }

  #[test]
  fn an_unchanged_frame_is_not_sent_again() {
    let (context, recorder) = recorded();
    play(&context, &solid("Dmd", 0, (255, 0, 0)));
    render(&context);
    render(&context);
    play(&context, &solid("Dmd", 1, (0, 0, 255)));
    render(&context);
    assert_eq!(recorder.frames(), [vec![RED; 8], vec![BLUE; 8]]);
  }

  #[test]
  fn edges_are_hard_unless_asked_otherwise() {
    // a circle whose edge passes through pixels rather than between them
    let circle = r#"Display(display: "Dmd", layer: (
      region: (base: Circle(radius: Fixed(1.3)), transform: [Translate(Fixed(2.0, 1.0))]),
      fill: (base: Solid(Fixed(255, 255, 255, 255))),
    ))"#;
    let partial = |frame: &[Rgb<u8>]| frame.iter().any(|Rgb([r, _, _])| *r != 0 && *r != 255);

    let (context, hard) = recorded();
    play(&context, circle);
    render(&context);
    assert!(!partial(&hard.frames()[0]), "{:?}", hard.frames());

    let soft = Recorder::default();
    let context = self::context(DisplayDefinition::new("Dmd", Device).soft_edges().mirror(soft.clone()));
    play(&context, circle);
    render(&context);
    assert!(partial(&soft.frames()[0]), "{:?}", soft.frames());
  }

  #[test]
  fn despawning_leaves_the_display_black() {
    let (context, recorder) = recorded();
    play(&context, &solid("Dmd", 0, (255, 0, 0)));
    render(&context);
    context.sys_ctx().systems.with::<DisplaySystem, _>(|displays| displays.on_despawn(&context.sys_ctx()));
    assert_eq!(recorder.frames(), [vec![RED; 8], vec![BLACK; 8]]);
  }
}
