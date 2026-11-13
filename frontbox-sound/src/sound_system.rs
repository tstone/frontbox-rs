use std::path::Path;

use frontbox_core::prelude::*;
use tokio::sync::mpsc;

use crate::CalloutCompleted;
use crate::callout::CalloutRequest;
use crate::sound_manager::SoundManagerMessage::*;
use crate::sound_manager::{OutputDevice, SoundManager, SoundManagerMessage};

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
}

impl SoundSystem {
  /// Play through the audio device whose name contains `device_name` on a hardware platform, or the computer's default
  /// output on a virtual one. The device is opened once the system spawns. Until it opens, and whenever it's lost after,
  /// it's tried again with a growing wait. Sounds can be preloaded and music asked for in the meantime, and carry over.
  pub fn new(device_name: &'static str) -> Self {
    let (tx, rx) = mpsc::unbounded_channel();
    let (completed_tx, completed_rx) = mpsc::unbounded_channel();
    Self {
      device_name,
      tx,
      completed_rx,
      manager_channels: Some((rx, completed_tx)),
      callouts_pending: 0,
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
  fn on_spawn(&mut self, ctx: &SystemContext) {
    let Some((rx, completed_tx)) = self.manager_channels.take() else {
      return;
    };
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
  }
}
