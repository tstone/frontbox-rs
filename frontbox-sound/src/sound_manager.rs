use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait};
use frontbox::prelude::*;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{
  AudioManager, AudioManagerSettings,
  backend::cpal::{CpalBackend, CpalBackendSettings, Error},
};
use kira::{Decibels, Tween};
use tokio::sync::{mpsc, watch};

pub struct SoundManager {
  rx: mpsc::UnboundedReceiver<SoundManagerCmds>,
  manager: AudioManager,
  sounds: HashMap<&'static str, StaticSoundData>,
  music_track: TrackHandle,
  callout_track: TrackHandle,
  // callouts can only play one at a time. If multiple are triggered in quick succession, they will be queued up and played one after another
  callout_queue: Vec<&'static str>,
  active_callout: Option<StaticSoundHandle>,
  active_music: Option<StaticSoundHandle>,
}

impl SoundManager {
  fn raw(mut manager: AudioManager, rx: mpsc::UnboundedReceiver<SoundManagerCmds>) -> Self {
    let music_track = manager.add_sub_track(TrackBuilder::default()).unwrap();
    let callout_track = manager.add_sub_track(TrackBuilder::default()).unwrap();

    Self {
      rx,
      manager,
      sounds: HashMap::new(),
      music_track,
      callout_track,
      active_callout: None,
      callout_queue: Vec::new(),
      active_music: None,
    }
  }

  pub fn new(
    device_name: &'static str,
    rx: mpsc::UnboundedReceiver<SoundManagerCmds>,
  ) -> Result<Self, Error> {
    let host = cpal::default_host();
    let devices: Vec<_> = host.output_devices().unwrap().collect();
    for device in &devices {
      log::trace!(
        target: "frontbox::sound",
        "Found audio device: {:?}",
        device.description().unwrap().extended()
      );
    }

    let device = devices.into_iter().find(|d| {
      let Ok(desc) = d.description() else {
        return false;
      };
      if !desc.name().contains(device_name) {
        return false;
      };
      desc.supports_output()
        && d.default_output_config().is_ok()
        && d.default_output_config().unwrap().sample_format() == SampleFormat::F32
    });

    if device.is_none() {
      log::warn!(
        target: "frontbox::sound",
        "Audio device matching '{}' not found, using system default",
        device_name
      );
    }

    let manager = AudioManager::<CpalBackend>::new(AudioManagerSettings {
      backend_settings: CpalBackendSettings {
        device,
        ..Default::default()
      },
      ..Default::default()
    })?;

    Ok(Self::raw(manager, rx))
  }

  pub async fn run(&mut self) {
    let (tick_tx, mut tick_rx) = watch::channel(());

    // Interval to check for ducking
    let mut timer_interval = tokio::time::interval(Duration::from_millis(100));
    tokio::spawn(async move {
      loop {
        timer_interval.tick().await;
        let _ = tick_tx.send(());
      }
    });

    loop {
      tokio::select! {
        Some(event) = self.rx.recv() => {
          match event {
            SoundManagerCmds::PreloadPath(name, path) => self.preload(name, path),
            SoundManagerCmds::PreloadEmbedded(name, bytes) => self.preload_embedded(name, bytes),
            SoundManagerCmds::PlaySfx(key) => self.play_sfx(key),
            SoundManagerCmds::PlayCallout(key) => self.play_callout(key),
            SoundManagerCmds::PlayMusic(path, crossfade) => self.crossfade_music(path, crossfade),
            SoundManagerCmds::StopMusic(crossfade) => self.stop_music(crossfade),
          }
        }

        Ok(_) = tick_rx.changed() => {
          self.on_tick();
        }
      }
    }
  }

  fn preload(&mut self, name: &'static str, path: impl AsRef<Path>) {
    log::debug!(target: "frontbox::sound", "Preloading sound '{}' at {:?}", name, path.as_ref());
    match StaticSoundData::from_file(path.as_ref()) {
      Ok(sound) => {
        self.sounds.insert(name, sound);
      }
      Err(e) => log::error!(
        target: "frontbox::sound",
        "Failed to preload sound {}: {:?}",
        path.as_ref().display(),
        e
      ),
    }
  }

  fn preload_embedded(&mut self, name: &'static str, bytes: &'static [u8]) {
    log::debug!(
      target: "frontbox::sound",
      "Preloading embedded sound '{}' ({} bytes)",
      name,
      bytes.len()
    );
    match StaticSoundData::from_cursor(Cursor::new(bytes)) {
      Ok(sound) => {
        self.sounds.insert(name, sound);
      }
      Err(e) => {
        log::error!(target: "frontbox::sound", "Failed to preload embedded sound {}: {:?}", name, e)
      }
    }
  }

  fn play_sfx(&mut self, key: &'static str) {
    if let Some(sound) = self.sounds.get(key) {
      log::debug!(target: "frontbox::sound", "🔊 Playing SFX '{}'", key);
      self.manager.play(sound.clone()).ok();
    } else {
      log::error!(target: "frontbox::sound", "🔊 Sound with key '{}' not found", key);
    }
  }

  fn play_callout(&mut self, key: &'static str) {
    if let Some(sound) = self.sounds.get(key) {
      if self.active_callout.is_some() {
        self.callout_queue.push(key);
      } else {
        Self::duck_track(&mut self.music_track, -10.0);
        self.play_callout_sound(sound.clone());
      }
    } else {
      log::error!(target: "frontbox::sound", "🗣️ Callout with key '{}' not found", key);
    }
  }

  pub fn play_callout_sound(&mut self, sound: StaticSoundData) {
    match self.callout_track.play(sound) {
      Ok(handle) => {
        self.active_callout = Some(handle);
      }
      Err(e) => log::error!(target: "frontbox::sound", "Failed to play callout sound: {:?}", e),
    }
  }

  fn crossfade_music(&mut self, path: impl AsRef<Path>, crossfade: Duration) {
    match StaticSoundData::from_file(path.as_ref()) {
      Ok(sound) => {
        log::info!(target: "frontbox::sound", "🎵 Playing music {}", path.as_ref().display());
        let new_music = self.music_track.play(sound).ok();
        if let Some(mut old_music) = self.active_music.replace(new_music.unwrap()) {
          old_music.stop(Tween {
            duration: crossfade,
            ..Default::default()
          });
        }
      }
      Err(e) => {
        log::error!(target: "frontbox::sound", "🎵 Failed to play music {}: {:?}", path.as_ref().display(), e)
      }
    }
  }

  fn stop_music(&mut self, crossfade: Duration) {
    if let Some(old_music) = &mut self.active_music {
      log::info!(target: "frontbox::sound", "🎵 Stopping music");
      old_music.stop(Tween {
        duration: crossfade,
        ..Default::default()
      });
    } else {
      log::info!(target: "frontbox::sound", "Stop music requested, but no music track was active");
    }
  }

  fn on_tick(&mut self) {
    let Self {
      active_callout,
      music_track,
      ..
    } = self;

    // un-duck music if callout finished
    if active_callout.is_some()
      && active_callout.as_ref().unwrap().state() == PlaybackState::Stopped
    {
      // check if there are any queued callouts, and if so play the next one instead of un-ducking music
      if let Some(next_key) = self.callout_queue.pop() {
        if let Some(sound) = self.sounds.get(next_key) {
          self.play_callout_sound(sound.clone());
        }
      } else {
        self.active_callout = None;
        Self::unduck_track(music_track);
      }
    }
  }

  fn duck_track(track: &mut TrackHandle, amount_db: f32) {
    track.set_volume(
      Decibels(amount_db),
      Tween {
        duration: Duration::from_millis(200),
        ..Default::default()
      },
    );
  }

  fn unduck_track(track: &mut TrackHandle) {
    track.set_volume(
      Decibels(0.0),
      Tween {
        duration: Duration::from_millis(200),
        ..Default::default()
      },
    );
  }
}

pub enum SoundManagerCmds {
  PreloadPath(&'static str, PathBuf),
  PreloadEmbedded(&'static str, &'static [u8]),
  PlaySfx(&'static str),
  PlayCallout(&'static str),
  PlayMusic(PathBuf, Duration),
  StopMusic(Duration),
}
