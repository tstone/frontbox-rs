use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

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

pub struct SoundSystem {
  manager: AudioManager,
  sounds: HashMap<&'static str, StaticSoundData>,
  music_track: TrackHandle,
  callout_track: TrackHandle,
  // callouts can only play one at a time. If multiple are triggered in quick succession, they will be queued up and played one after another
  callout_queue: Vec<&'static str>,
  active_callout: Option<StaticSoundHandle>,
  active_music: Option<StaticSoundHandle>,
}

impl SoundSystem {
  pub fn raw(mut manager: AudioManager) -> Self {
    let music_track = manager.add_sub_track(TrackBuilder::default()).unwrap();
    let callout_track = manager.add_sub_track(TrackBuilder::default()).unwrap();

    Self {
      manager,
      sounds: HashMap::new(),
      music_track,
      callout_track,
      active_callout: None,
      callout_queue: Vec::new(),
      active_music: None,
    }
  }

  #[allow(unused)]
  fn default() -> Result<Self, Error> {
    AudioManager::<CpalBackend>::new(AudioManagerSettings {
      backend_settings: CpalBackendSettings {
        device: None,
        ..Default::default()
      },
      ..Default::default()
    })
    .map(Self::raw)
  }

  #[allow(unused)]
  pub fn by_name(device_name: &'static str) -> Result<Self, Error> {
    let host = cpal::default_host();
    let devices: Vec<_> = host.output_devices().unwrap().collect();
    for device in &devices {
      log::trace!(
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

    Ok(Self::raw(manager))
  }

  pub fn preload(&mut self, name: &'static str, path: impl AsRef<Path>) {
    log::debug!("Preloading sound '{}' at {:?}", name, path.as_ref());
    match StaticSoundData::from_file(path.as_ref()) {
      Ok(sound) => {
        self.sounds.insert(name, sound);
      }
      Err(e) => log::error!(
        "Failed to preload sound {}: {:?}",
        path.as_ref().display(),
        e
      ),
    }
  }

  pub fn preload_embedded(&mut self, name: &'static str, bytes: &'static [u8]) {
    log::debug!(
      "Preloading embedded sound '{}' ({} bytes)",
      name,
      bytes.len()
    );
    match StaticSoundData::from_cursor(Cursor::new(bytes)) {
      Ok(sound) => {
        self.sounds.insert(name, sound);
      }
      Err(e) => log::error!("Failed to preload embedded sound {}: {:?}", name, e),
    }
  }

  /// Play a pre-loaded wave file once all the way through
  pub fn play_sfx(&mut self, key: &'static str) {
    if let Some(sound) = self.sounds.get(key) {
      self.manager.play(sound.clone()).ok();
    } else {
      log::error!("Sound with key '{}' not found", key);
    }
  }

  /// Play the wave AND duck any music that is currently playing, then restore the music volume after the sound finishes
  pub fn play_callout(&mut self, key: &'static str) {
    // TODO: does this need to queue them up if multiple callouts are played in quick succession? Or just stop the previous one and start the new one?
    if let Some(sound) = self.sounds.get(key) {
      if self.active_callout.is_some() {
        self.callout_queue.push(key);
      } else {
        Self::duck_track(&mut self.music_track, -10.0);
        self.play_callout_sound(sound.clone());
      }
    } else {
      log::error!("Sound with key '{}' not found", key);
    }
  }

  pub fn play_callout_sound(&mut self, sound: StaticSoundData) {
    match self.callout_track.play(sound) {
      Ok(handle) => {
        self.active_callout = Some(handle);
      }
      Err(e) => log::error!("Failed to play callout sound: {:?}", e),
    }
  }

  // TODO: looping
  pub fn play_music(&mut self, path: impl AsRef<Path>) {
    match StaticSoundData::from_file(path.as_ref()) {
      Ok(sound) => {
        self.active_music = self.music_track.play(sound).ok();
      }
      Err(e) => log::error!("Failed to play music {}: {:?}", path.as_ref().display(), e),
    }
  }

  pub fn stop_music(&mut self, fade_down: Duration) {
    if let Some(old_music) = &mut self.active_music {
      old_music.stop(Tween {
        duration: fade_down,
        ..Default::default()
      });
    }
  }

  /// Changes the active music track to the new one, fading out the old one and fading in the new one
  pub fn crossfade_music(&mut self, next_path: impl AsRef<Path>, duration: Duration) {
    match StaticSoundData::from_file(next_path.as_ref()) {
      Ok(sound) => {
        let new_music = self.music_track.play(sound).ok();
        if let Some(mut old_music) = self.active_music.replace(new_music.unwrap()) {
          old_music.stop(Tween {
            duration,
            ..Default::default()
          });
        }
      }
      Err(e) => log::error!(
        "Failed to play music {}: {:?}",
        next_path.as_ref().display(),
        e
      ),
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

impl System for SoundSystem {
  fn on_tick(&mut self, _delta: Duration, _ctx: &SystemContext) {
    let SoundSystem {
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
}
