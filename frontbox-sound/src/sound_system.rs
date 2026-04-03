use std::path::Path;

use cpal::traits::{DeviceTrait, HostTrait};
use frontbox::prelude::*;
use kira::sound::static_sound::StaticSoundData;
use kira::{
  AudioManager, AudioManagerSettings,
  backend::cpal::{CpalBackend, CpalBackendSettings, Error},
};

pub struct SoundSystem {
  manager: AudioManager,
}

impl SoundSystem {
  pub fn raw(manager: AudioManager) -> Self {
    Self { manager }
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
    .map(|manager| Self::raw(manager))
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
      d.default_output_config().is_ok()
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

  /// Play a wave file once all the way through
  pub fn play_sfx(&mut self, path: impl AsRef<Path>) {
    match StaticSoundData::from_file(path.as_ref()) {
      Ok(sound) => {
        self.manager.play(sound).ok();
      }
      Err(e) => log::error!("Failed to play sound {}: {:?}", path.as_ref().display(), e),
    }
  }
}

impl System for SoundSystem {}
