use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use cpal::StreamError;
use frontbox_core::prelude::*;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::{TrackBuilder, TrackHandle};
use kira::{AudioManager, Decibels, ResourceLimitReached, StartTime, Tween};

use crate::SoundKey;
use crate::callout::CalloutRequest;

/// How far the music drops while a callout plays
const CALLOUT_DUCK_DB: f32 = -12.0;
/// How long a sound cut off before its end takes to fade out, or one started partway in to fade in, as starting or
/// stopping one at once clicks
const CUT_FADE: Duration = Duration::from_millis(15);
/// Kira hears from the audio driver every few milliseconds while the stream runs. This long without hearing from it means
/// the stream has died, whether or not it reported an error.
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(1);

/// An open audio device, and everything on it. All of it goes when the device does.
pub(super) struct Output {
  manager: AudioManager,
  music_track: TrackHandle,
  callout_track: TrackHandle,
  active_music: Option<StaticSoundHandle>,
  /// The line being said. The [`SoundManager`](super::SoundManager) moves through its queue with this.
  pub active_callout: Option<ActiveCallout>,
  /// When kira last heard from the audio driver
  last_heartbeat: Instant,
}

impl Output {
  pub fn new(
    mut manager: AudioManager,
    callout_volume: f32,
    now: Instant,
  ) -> Result<Self, ResourceLimitReached> {
    let music_track = manager.add_sub_track(TrackBuilder::default())?;
    let callout_track =
      manager.add_sub_track(TrackBuilder::default().volume(Decibels(callout_volume)))?;
    Ok(Self {
      manager,
      music_track,
      callout_track,
      active_music: None,
      active_callout: None,
      last_heartbeat: now,
    })
  }

  /// Whether the stream is still playing, and why not if it isn't
  pub fn check_health(&mut self, now: Instant) -> Result<(), String> {
    let backend = self.manager.backend_mut();
    while let Some(error) = backend.pop_error() {
      match error {
        // kira restarts the stream itself on these, but on the system default device rather than the one chosen, and
        // panics its stream thread if that fails. Opening the output again is the only way to be sure of it.
        StreamError::DeviceNotAvailable | StreamError::StreamInvalidated => {
          return Err(error.to_string());
        }
        StreamError::BufferUnderrun => {
          log::trace!(target: "frontbox::sound", "Audio buffer underrun")
        }
        StreamError::BackendSpecific { .. } => {
          log::warn!(target: "frontbox::sound", "Audio stream error: {}", error)
        }
      }
    }

    // kira reports its CPU use each time the driver asks it for audio, which makes it a heartbeat for the stream
    while backend.pop_cpu_usage().is_some() {
      self.last_heartbeat = now;
    }
    let silent_for = now.duration_since(self.last_heartbeat);
    if silent_for >= HEARTBEAT_TIMEOUT {
      return Err(format!("no audio asked for in {:?}", silent_for));
    }
    Ok(())
  }

  /// Play a sound at `volume`, where 1.0 is as recorded, from `skip` into it, and fade it out by `length` if given
  pub fn play_sfx(&mut self, key: &str, sound: StaticSoundData, volume: f32, skip: Duration, length: Option<Duration>) {
    log::debug!(target: "frontbox::sound", "🔊 Playing SFX '{}'", key);
    let mut sound = sound.volume(amplitude_to_db(volume));
    if !skip.is_zero() {
      // starting partway in clicks as stopping early does
      sound = sound.start_position(skip.as_secs_f64()).fade_in_tween(Tween { duration: CUT_FADE, ..Default::default() });
    }
    match self.manager.play(sound) {
      Ok(mut handle) => {
        if let Some(length) = length {
          let fade = CUT_FADE.min(length);
          handle.stop(Tween { start_time: StartTime::Delayed(length - fade), duration: fade, ..Default::default() });
        }
      }
      Err(e) => log::error!(target: "frontbox::sound", "Failed to play SFX '{}': {:?}", key, e),
    }
  }

  /// Start a line, with its lead-in unless it follows on from a line that already had it. Returns the handle of the
  /// line's last sound, which says when the line is over.
  pub fn start_callout(
    &mut self,
    sounds: &HashMap<SoundKey, StaticSoundData>,
    request: &CalloutRequest,
    follows_on: bool,
  ) -> Option<StaticSoundHandle> {
    let Some(sound) = sounds.get(request.key()).cloned() else {
      log::error!(target: "frontbox::sound", "🗣️ Callout with key '{}' not found", request.key());
      return None;
    };
    let mut line = vec![sound];
    for key in request.then_keys() {
      match sounds.get(*key) {
        Some(sound) => line.push(sound.clone()),
        None => {
          log::error!(target: "frontbox::sound", "🗣️ Callout with key '{}' not found, in line '{}'", key, request.key())
        }
      }
    }

    // each sound is scheduled on the audio clock, as waiting for a tick to start it would leave a gap of up to 100ms
    let mut delay = Duration::ZERO;
    let lead_in = request
      .lead_in_key()
      .filter(|_| !follows_on)
      .and_then(|key| sounds.get(key))
      .cloned();
    if let Some(lead_in) = lead_in {
      delay = lead_in.duration();
      if let Err(e) = self.callout_track.play(lead_in) {
        log::error!(target: "frontbox::sound", "Failed to play callout lead-in: {:?}", e);
      }
    }

    let mut last = None;
    for sound in line {
      let duration = sound.duration();
      match self
        .callout_track
        .play(sound.start_time(StartTime::Delayed(delay)))
      {
        Ok(handle) => last = Some(handle),
        Err(e) => {
          log::error!(target: "frontbox::sound", "Failed to play callout sound: {:?}", e)
        }
      }
      delay += duration;
    }
    last
  }

  pub fn set_callout_volume(&mut self, db: f32) {
    self
      .callout_track
      .set_volume(Decibels(db), Tween::default());
  }

  /// Crossfade into the music at `path`. Returns whether it plays.
  pub fn start_music(&mut self, path: &Path, crossfade: Duration) -> bool {
    let sound = match StaticSoundData::from_file(path) {
      Ok(sound) => sound,
      Err(e) => {
        log::error!(target: "frontbox::sound", "🎵 Failed to load music {}: {:?}", path.display(), e);
        return false;
      }
    };
    match self.music_track.play(sound) {
      Ok(music) => {
        log::info!(target: "frontbox::sound", "🎵 Playing music {}", path.display());
        if let Some(mut old_music) = self.active_music.replace(music) {
          old_music.stop(Tween {
            duration: crossfade,
            ..Default::default()
          });
        }
        true
      }
      Err(e) => {
        log::error!(target: "frontbox::sound", "🎵 Failed to play music {}: {:?}", path.display(), e);
        false
      }
    }
  }

  /// Fade out the music. Returns whether there was any to stop.
  pub fn stop_music(&mut self, crossfade: Duration) -> bool {
    let Some(music) = &mut self.active_music else {
      return false;
    };
    music.stop(Tween {
      duration: crossfade,
      ..Default::default()
    });
    true
  }

  /// Whether the last music started has played out. A dead stream leaves a track as it was, so music cut off by a lost
  /// device hasn't.
  pub fn music_played_out(&self) -> bool {
    self
      .active_music
      .as_ref()
      .is_some_and(|music| music.state() == PlaybackState::Stopped)
  }

  pub fn duck_music(&mut self) {
    self.music_track.set_volume(
      Decibels(CALLOUT_DUCK_DB),
      Tween {
        duration: Duration::from_millis(200),
        ..Default::default()
      },
    );
  }

  pub fn unduck_music(&mut self) {
    self.music_track.set_volume(
      Decibels(0.0),
      Tween {
        duration: Duration::from_millis(200),
        ..Default::default()
      },
    );
  }
}

pub(super) struct ActiveCallout {
  pub key: &'static str,
  pub lead_in: Option<&'static str>,
  pub handle: StaticSoundHandle,
}

/// A volume where 1.0 is as recorded, in decibels. Nothing is silence.
fn amplitude_to_db(amplitude: f32) -> Decibels {
  match amplitude > 0.0 {
    true => Decibels(20.0 * amplitude.log10()),
    false => Decibels::SILENCE,
  }
}
