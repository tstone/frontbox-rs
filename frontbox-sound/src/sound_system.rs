use std::path::Path;

use frontbox::prelude::*;
use kira::backend::cpal::Error;
use tokio::sync::mpsc;

use crate::sound_manager::SoundManagerCmds::*;
use crate::sound_manager::{SoundManager, SoundManagerCmds};

pub struct SoundSystem {
  tx: mpsc::UnboundedSender<SoundManagerCmds>
}

impl SoundSystem {
  pub fn by_name(device_name: &'static str) -> Result<Self, Error> {
    let (tx, rx) = mpsc::unbounded_channel::<SoundManagerCmds>();
    match SoundManager::new(device_name, rx) {
      Ok(mut manager) => {
        tokio::spawn(async move {
          manager.run().await;
        });
        Ok(Self { tx })
      },
      Err(err) => Err(err)
    }
  }

  pub fn preload(&mut self, name: &'static str, path: impl AsRef<Path>) {
    let _ = self.tx.send(PreloadPath(name, path.as_ref().to_path_buf()));
  }

  pub fn preload_embedded(&mut self, name: &'static str, bytes: &'static [u8]) {
    let _ = self.tx.send(PreloadEmbedded(name, bytes));
  }

  /// Play a pre-loaded wave file once all the way through
  pub fn play_sfx(&mut self, key: &'static str) {
    let _ = self.tx.send(PlaySfx(key));
  }

  /// Play the wave AND duck any music that is currently playing, then restore the music volume after the sound finishes
  pub fn play_callout(&mut self, key: &'static str) {
    let _ = self.tx.send(PlayCallout(key));
  }

  pub fn play_music(&mut self, path: impl AsRef<Path>, crossfade: Duration) {
    let _ = self.tx.send(PlayMusic(path.as_ref().to_path_buf(), crossfade));
  }

  pub fn stop_music(&mut self, crossfade: Duration) {
    let _ = self.tx.send(StopMusic(crossfade));
  }
}

impl System for SoundSystem {}

// impl System for SoundSystem {
//   fn on_tick(&mut self, _delta: Duration, _ctx: &Context) {
//     let SoundSystem {
//       active_callout,
//       music_track,
//       ..
//     } = self;

//     // un-duck music if callout finished
//     if active_callout.is_some()
//       && active_callout.as_ref().unwrap().state() == PlaybackState::Stopped
//     {
//       // check if there are any queued callouts, and if so play the next one instead of un-ducking music
//       if let Some(next_key) = self.callout_queue.pop() {
//         if let Some(sound) = self.sounds.get(next_key) {
//           self.play_callout_sound(sound.clone());
//         }
//       } else {
//         self.active_callout = None;
//         Self::unduck_track(music_track);
//       }
//     }
//   }
// }
