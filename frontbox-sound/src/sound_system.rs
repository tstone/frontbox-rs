use cpal::traits::{DeviceTrait, HostTrait};
use frontbox::prelude::*;
use kira::sound::static_sound::StaticSoundData;
use kira::{
  AudioManager, AudioManagerSettings,
  backend::{
    Backend,
    cpal::{CpalBackend, CpalBackendSettings, Error},
  },
};

pub struct SoundSystem {
  manager: AudioManager,
}

impl SoundSystem {
  pub fn raw(manager: AudioManager) -> Self {
    Self { manager }
  }

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

  pub fn by_name(device_name: &'static str) -> Result<Self, Error> {
    let host = cpal::default_host();
    for device in host.output_devices().unwrap() {
      log::trace!(
        "Found audio device: {:?}",
        device.description().unwrap().extended()
      );
    }

    let device = host.output_devices().unwrap().find(|d| {
      d.description()
        .map(|d| {
          d.name().contains(device_name)
            && d
              .extended()
              .get(1)
              .map(|desc| desc.contains("all software conversions"))
              .unwrap_or(false)
        })
        .unwrap_or(false)
    });

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
  pub fn play_sfx(&mut self, path: &'static str) {
    match StaticSoundData::from_file(path) {
      Ok(sound) => {
        self.manager.play(sound).ok();
      }
      Err(e) => log::error!("Failed to play sound {}: {:?}", path, e),
    }
  }
}

impl System for SoundSystem {}
