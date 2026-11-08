use std::path::Path;

use frontbox_core::prelude::*;
use kira::backend::cpal::Error;
use tokio::sync::mpsc;

use crate::CalloutCompleted;
use crate::callout::CalloutRequest;
use crate::sound_manager::SoundManagerCmds::*;
use crate::sound_manager::{SoundManager, SoundManagerCmds};

pub struct SoundSystem {
  tx: mpsc::UnboundedSender<SoundManagerCmds>,
  completed_rx: mpsc::UnboundedReceiver<CalloutCompleted>,
  /// Lines asked for that haven't been reported done with
  callouts_pending: usize,
}

impl SoundSystem {
  pub fn by_name(device_name: &'static str) -> Result<Self, Error> {
    let (tx, rx) = mpsc::unbounded_channel::<SoundManagerCmds>();
    let (completed_tx, completed_rx) = mpsc::unbounded_channel();
    match SoundManager::new(device_name, rx, completed_tx) {
      Ok(mut manager) => {
        tokio::spawn(async move {
          manager.run().await;
        });
        Ok(Self {
          tx,
          completed_rx,
          callouts_pending: 0,
        })
      }
      Err(err) => Err(err),
    }
  }

  pub fn default() -> Result<Self, Error> {
    let (tx, rx) = mpsc::unbounded_channel::<SoundManagerCmds>();
    let (completed_tx, completed_rx) = mpsc::unbounded_channel();
    match SoundManager::default(rx, completed_tx) {
      Ok(mut manager) => {
        tokio::spawn(async move {
          manager.run().await;
        });
        Ok(Self {
          tx,
          completed_rx,
          callouts_pending: 0,
        })
      }
      Err(err) => Err(err),
    }
  }

  pub fn by_name_or_default(device_name: &'static str) -> Self {
    match Self::by_name(device_name) {
      Ok(system) => system,
      Err(_) => Self::default().expect("Could not initialize SoundSystem"),
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
}

impl System for SoundSystem {
  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    while let Ok(completed) = self.completed_rx.try_recv() {
      self.callouts_pending = self.callouts_pending.saturating_sub(1);
      ctx.emit(completed);
    }
  }
}
