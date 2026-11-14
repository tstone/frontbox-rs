use std::collections::{HashMap, HashSet};
use std::path::Path;

use frontbox_core::prelude::app_tracer::SoundPreviewRequested;
use frontbox_core::prelude::*;
use frontbox_gfx::scene::Content;
use frontbox_scene::{PlaybackId, Scene, SceneSystem};
use tokio::sync::mpsc;

use crate::callout::CalloutRequest;
use crate::sound_manager::SoundManagerMessage::*;
use crate::sound_manager::{OutputDevice, SoundManager, SoundManagerMessage};
use crate::{CalloutCompleted, SoundKey};

pub struct SoundSystem {
  device_name: &'static str,
  tx: mpsc::UnboundedSender<SoundManagerMessage>,
  completed_rx: mpsc::UnboundedReceiver<CalloutCompleted>,
  /// The manager's ends of the channels, until the system spawns and knows which device to play on
  manager_channels: Option<(
    mpsc::UnboundedReceiver<SoundManagerMessage>,
    mpsc::UnboundedSender<CalloutCompleted>,
  )>,
  /// Lines asked for that haven't been reported done with
  callouts_pending: usize,
  /// Where scenes' sounds are loaded from, once the system spawns. `None` when the app has no content folder.
  content: Option<Content>,
  /// Sounds from the content folder that have been asked to load, by name
  content_loaded: HashSet<String>,
  /// Sounds from the content folder asked for before the system spawned and knew where the folder is
  load_on_spawn: Vec<String>,
  /// The scene time each playback was at on the last tick, so each start it passes is played once
  playback_times: HashMap<PlaybackId, f32>,
}

impl SoundSystem {
  /// Play through the audio device whose name contains `device_name` on a hardware platform, or the computer's default
  /// output on a virtual one. The device is opened once the system spawns. Until it opens, and whenever it's lost after,
  /// it's tried again with a growing wait. Sounds can be preloaded and music asked for in the meantime, and carry over.
  ///
  /// It also plays the sound layers of the scenes the `SceneSystem` shows, loading their sounds from the app's content
  /// folder.
  pub fn new(device_name: &'static str) -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let (completed_tx, completed_rx) = mpsc::unbounded_channel();
    Self {
      device_name,
      tx,
      completed_rx,
      manager_channels: Some((rx, completed_tx)),
      callouts_pending: 0,
      content: None,
      content_loaded: HashSet::new(),
      load_on_spawn: Vec::new(),
      playback_times: HashMap::new(),
    }
  }

  pub fn preload(&mut self, name: impl Into<SoundKey>, path: impl AsRef<Path>) {
    let _ = self.tx.send(PreloadPath(name.into(), path.as_ref().to_path_buf()));
  }

  pub fn preload_embedded(&mut self, name: impl Into<SoundKey>, bytes: &'static [u8]) {
    let _ = self.tx.send(PreloadEmbedded(name.into(), bytes));
  }

  /// Load the sounds a scene plays from the content folder, so the first of them isn't late while it loads. A
  /// scene's sounds are otherwise loaded the first time it plays.
  pub fn preload_scene(&mut self, scene: &Scene) {
    for name in scene.sound_names() {
      match self.manager_channels.is_some() {
        true => self.load_on_spawn.push(name.to_string()),
        false => _ = self.load_from_content(name),
      }
    }
  }

  /// Play a pre-loaded wave file once all the way through
  pub fn play_sfx(&mut self, key: impl Into<SoundKey>) {
    let _ = self.tx.send(PlaySfx(key.into(), 1.0, Duration::ZERO, None));
  }

  /// Play the wave AND duck any music that is currently playing, then restore the music volume after the sound finishes.
  /// Lines play one at a time, and [`CalloutCompleted`] says when this one is done with.
  pub fn play_callout(&mut self, request: impl Into<CalloutRequest>) {
    self.callouts_pending += 1;
    let _ = self.tx.send(PlayCallout(request.into()));
  }

  /// Whether a line is being said or waiting to be, to keep other sounds from talking over it. Known as of the last
  /// tick, so a line that just ended can still count for up to one.
  pub fn callout_pending(&self) -> bool {
    self.callouts_pending > 0
  }

  /// Turn callouts up or down relative to everything else, in decibels. 0 is as recorded, and above it boosts them, which
  /// can clip loud lines.
  pub fn set_callout_volume(&mut self, db: f32) {
    let _ = self.tx.send(SetCalloutVolume(db));
  }

  pub fn play_music(&mut self, path: impl AsRef<Path>, crossfade: Duration) {
    let _ = self
      .tx
      .send(PlayMusic(path.as_ref().to_path_buf(), crossfade));
  }

  pub fn stop_music(&mut self, crossfade: Duration) {
    let _ = self.tx.send(StopMusic(crossfade));
  }

  /// Play a sound from the content folder at `volume`, from `skip` seconds in, and cut off after `length` seconds if
  /// given, loading it first if it hasn't been
  fn play_from_content(&mut self, name: &str, volume: f32, skip: f32, length: Option<f32>) {
    if self.load_from_content(name) {
      let length = length.map(|length| Duration::from_secs_f32(length.max(0.0)));
      let skip = Duration::from_secs_f32(skip.max(0.0));
      let _ = self.tx.send(PlaySfx(SoundKey::Owned(name.to_string()), volume, skip, length));
    }
  }

  /// Ask for a sound from the content folder to be loaded, unless it already has been. Returns whether it can be
  /// played.
  fn load_from_content(&mut self, name: &str) -> bool {
    if self.content_loaded.contains(name) {
      return true;
    }
    let Some(content) = &self.content else {
      log::error!(target: "frontbox::sound", "🔊 Can't play scene sound '{}': the app has no content folder", name);
      return false;
    };
    match content.sound_path(name) {
      Ok(path) => {
        self.content_loaded.insert(name.to_string());
        let _ = self.tx.send(PreloadPath(SoundKey::Owned(name.to_string()), path));
        true
      }
      Err(err) => {
        log::error!(target: "frontbox::sound", "🔊 Can't play scene sound: {}", err);
        false
      }
    }
  }

  /// Play the sounds each playback shown has passed the start of since the last tick
  fn play_scenes(&mut self, ctx: &SystemContext) {
    let Some(scenes) = ctx.systems.get::<SceneSystem>() else {
      self.playback_times.clear();
      return;
    };
    let mut started = Vec::new();
    for (id, playback) in scenes.shown() {
      let moment = playback.moment();
      // a playback first seen here has just started, so plays what's right at its top
      let after = self.playback_times.insert(id, moment.time);
      let sounds = playback.scene().sounds_started(&moment, after);
      started.extend(sounds.into_iter().map(|start| (start.sound.to_string(), start.volume, start.skip, start.length)));
    }
    // a paused playback keeps its time, so it doesn't play its top again when it's shown again
    self.playback_times.retain(|id, _| scenes.contains(*id));
    drop(scenes);

    for (sound, volume, skip, length) in started {
      self.play_from_content(&sound, volume, skip, length);
    }
  }
}

impl System for SoundSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.content = ctx.content_path.clone().map(Content::new);
    let Some((rx, completed_tx)) = self.manager_channels.take() else {
      return;
    };
    for name in std::mem::take(&mut self.load_on_spawn) {
      self.load_from_content(&name);
    }
    let device = match ctx.platform {
      PlatformKind::Hardware => OutputDevice::Named(self.device_name),
      PlatformKind::Virtual => OutputDevice::SystemDefault,
    };
    tokio::spawn(SoundManager::new(device, rx, completed_tx).run());
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    while let Ok(completed) = self.completed_rx.try_recv() {
      self.callouts_pending = self.callouts_pending.saturating_sub(1);
      ctx.emit(completed);
    }
    self.play_scenes(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if let Some(SoundPreviewRequested { sound, volume, skip, length }) = event.downcast_ref::<SoundPreviewRequested>() {
      self.play_from_content(sound, *volume, *skip, *length);
    }
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::scene::Timing;
  use frontbox_scene::{PlayableScene, ScenePlayer, SoundLayer};

  use super::*;


  /// A system as if spawned, with the manager's end of its channel to see what it asks for
  fn context() -> (TestContext, mpsc::UnboundedReceiver<SoundManagerMessage>) {
    let mut sound = SoundSystem::new("test");
    let (rx, _) = sound.manager_channels.take().unwrap();
    sound.content = Some(Content::new("content"));
    let mut context = TestContext::default();
    context.insert_system(SceneSystem::new());
    context.insert_system(sound);
    (context, rx)
  }

  fn scene(cycle: Cycle) -> PlayableScene {
    let layers = vec![SoundLayer::new("sounds/a.wav").volume(0.5).into(), SoundLayer::new("sounds/b.wav").volume(0.5).start(0.5).skip(0.125).end(0.75).into()];
    Scene::new(layers).timing(Timing::new(1.0, cycle)).into()
  }

  /// Moves scenes on by `delta`, then has the sound system look at them
  fn tick(context: &TestContext, delta: Duration) {
    let ctx = context.sys_ctx();
    ctx.systems.with::<SceneSystem, _>(|scenes| scenes.on_tick(delta, &ctx));
    ctx.systems.with::<SoundSystem, _>(|sound| sound.on_tick(delta, &ctx));
  }

  /// The sounds asked to play since the last call
  fn played(rx: &mut mpsc::UnboundedReceiver<SoundManagerMessage>) -> Vec<(String, f32, Duration, Option<Duration>)> {
    let mut played = Vec::new();
    while let Ok(message) = rx.try_recv() {
      if let PlaySfx(key, volume, skip, length) = message {
        played.push((key.into_owned(), volume, skip, length));
      }
    }
    played
  }

  #[test]
  fn a_scene_plays_each_sound_as_its_clock_passes_it() {
    let (context, mut rx) = context();
    let mut player = ScenePlayer::new(scene(Cycle::Once));
    player.start(&context.sys_ctx());

    tick(&context, Duration::ZERO);
    assert_eq!(played(&mut rx), [("sounds/a.wav".to_string(), 0.5, Duration::ZERO, None)]);
    tick(&context, Duration::from_millis(400));
    assert!(played(&mut rx).is_empty());
    tick(&context, Duration::from_millis(200));
    assert_eq!(played(&mut rx), [("sounds/b.wav".to_string(), 0.5, Duration::from_millis(125), Some(Duration::from_millis(250)))], "from partway in, cut off at its end");
  }

  #[test]
  fn each_cycle_plays_its_sounds_again() {
    let (context, mut rx) = context();
    ScenePlayer::new(scene(Cycle::Times(2))).start(&context.sys_ctx());
    for _ in 0..25 {
      tick(&context, Duration::from_millis(100));
    }
    let sounds: Vec<_> = played(&mut rx).into_iter().map(|(sound, ..)| sound).collect();
    assert_eq!(sounds, ["sounds/a.wav", "sounds/b.wav", "sounds/a.wav", "sounds/b.wav"]);
  }

  #[test]
  fn a_playback_shown_again_doesnt_start_over() {
    let (context, mut rx) = context();
    ScenePlayer::new(scene(Cycle::Forever)).start(&context.sys_ctx());
    tick(&context, Duration::from_millis(200));
    played(&mut rx);

    let ctx = context.sys_ctx();
    ctx.systems.with::<SceneSystem, _>(|scenes| scenes.on_event(&SystemDeactivated::new(ctx.id(), ROOT_GROUP), &ctx));
    tick(&context, Duration::from_millis(500));
    ctx.systems.with::<SceneSystem, _>(|scenes| scenes.on_event(&SystemReactivated::new(ctx.id(), ROOT_GROUP), &ctx));
    tick(&context, Duration::ZERO);
    assert!(played(&mut rx).is_empty());
  }

  #[test]
  fn a_sound_is_loaded_from_the_content_folder_once() {
    let (context, mut rx) = context();
    for _ in 0..2 {
      context.sys_ctx().systems.with::<SoundSystem, _>(|sound| sound.play_from_content("sounds/a.wav", 1.0, 0.0, None));
    }
    let mut loads = 0;
    while let Ok(message) = rx.try_recv() {
      if let PreloadPath(key, path) = message {
        assert_eq!((key.as_ref(), path), ("sounds/a.wav", Path::new("content").join("sounds/a.wav")));
        loads += 1;
      }
    }
    assert_eq!(loads, 1);
  }

  #[test]
  fn a_console_preview_plays_its_sound() {
    let (context, mut rx) = context();
    let ctx = context.sys_ctx();
    let preview = SoundPreviewRequested { sound: "sounds/b.wav".into(), volume: 0.25, skip: 0.25, length: Some(0.5) };
    ctx.systems.with::<SoundSystem, _>(|sound| sound.on_event(&preview, &ctx));
    assert_eq!(played(&mut rx), [("sounds/b.wav".to_string(), 0.25, Duration::from_millis(250), Some(Duration::from_millis(500)))]);
  }
}
