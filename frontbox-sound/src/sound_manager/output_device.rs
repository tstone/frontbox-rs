use cpal::SampleFormat;
use cpal::traits::{DeviceTrait, HostTrait};
use kira::backend::cpal::{CpalBackend, CpalBackendSettings};
use kira::{AudioManager, AudioManagerSettings};

/// Which audio device to play on
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputDevice {
  /// The first device whose name contains this that can be opened. No other device is used in its place.
  Named(&'static str),
  /// Whatever the computer's default output is
  SystemDefault,
}

impl OutputDevice {
  /// Find and open the device. Blocks, as listing and opening devices waits on the audio driver.
  pub(super) fn open(self) -> Result<Opened, String> {
    let host = cpal::default_host();
    match self {
      OutputDevice::SystemDefault => {
        let default = host
          .default_output_device()
          .ok_or("there's no default output device")?;
        let name = device_name(&default);
        // left to kira to find, so on Linux and Windows it follows the default when that changes
        let manager = open_manager(None).map_err(|e| format!("'{}': {}", name, e))?;
        Ok(Opened { manager, name })
      }
      OutputDevice::Named(wanted) => {
        let devices = host
          .output_devices()
          .map_err(|e| format!("couldn't list output devices: {}", e))?;
        let matching: Vec<_> = devices
          .filter(|device| {
            device.description().is_ok_and(|description| {
              log::trace!(
                target: "frontbox::sound",
                "Found audio device: {:?}",
                description.extended()
              );
              description.name().contains(wanted) && description.supports_output()
            })
          })
          .collect();
        if matching.is_empty() {
          return Err(format!("no device named like '{}'", wanted));
        }

        // a card can be listed more than once, as raw hardware and through conversions. Kira renders f32, so only the
        // ones that take it will open, and a device still held by another process can't report a config at all.
        let mut reasons = Vec::new();
        for device in matching {
          let name = device_name(&device);
          match device.default_output_config() {
            Ok(config) if config.sample_format() == SampleFormat::F32 => {}
            Ok(config) => {
              reasons.push(format!("'{}' takes {} samples, not f32", name, config.sample_format()));
              continue;
            }
            Err(e) => {
              reasons.push(format!("'{}': {}", name, e));
              continue;
            }
          }
          match open_manager(Some(device)) {
            Ok(manager) => return Ok(Opened { manager, name }),
            Err(e) => reasons.push(format!("'{}': {}", name, e)),
          }
        }
        Err(reasons.join("; "))
      }
    }
  }
}

/// A device that opened
pub(super) struct Opened {
  pub manager: AudioManager,
  /// The device's name, for the logs
  pub name: String,
}

fn open_manager(device: Option<cpal::Device>) -> Result<AudioManager, kira::backend::cpal::Error> {
  AudioManager::<CpalBackend>::new(AudioManagerSettings {
    backend_settings: CpalBackendSettings {
      device,
      ..Default::default()
    },
    ..Default::default()
  })
}

fn device_name(device: &cpal::Device) -> String {
  device
    .description()
    .map(|description| description.name().to_string())
    .unwrap_or_else(|_| "unnamed device".into())
}
