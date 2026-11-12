use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::Instant;

use frontbox_core::prelude::*;
use kira::sound::PlaybackState;
use kira::sound::static_sound::StaticSoundData;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::time::MissedTickBehavior;

use super::connection::{Connection, retry_wait};
use super::output::{ActiveCallout, Output};
use super::output_device::{Opened, OutputDevice};
use super::sound_manager_message::SoundManagerMessage;
use crate::CalloutCompleted;
use crate::callout::{CalloutQueue, CalloutRequest};

/// How often the manager checks on callouts and the output
const TICK: Duration = Duration::from_millis(100);

/// Plays the sounds [`SoundSystem`](crate::SoundSystem) asks for. It owns the connection to the audio device, opening
/// it in the background, retrying with a growing wait until it can, and opening it again whenever it's lost. What the
/// game has asked for (preloaded sounds, callout volume, the music) is kept here rather than on the connection, so it
/// carries over to a new one.
pub(crate) struct SoundManager {
  device: OutputDevice,
  rx: mpsc::UnboundedReceiver<SoundManagerMessage>,
  completed_tx: mpsc::UnboundedSender<CalloutCompleted>,
  sounds: HashMap<&'static str, StaticSoundData>,
  callout_volume: f32,
  /// The music asked for, so it can be started again on a new connection
  music: Option<PathBuf>,
  // callouts can only play one at a time. If multiple are triggered in quick succession, they will be queued up and played one after another
  callout_queue: CalloutQueue,
  connection: Connection,
}

impl SoundManager {
  pub fn new(
    device: OutputDevice,
    rx: mpsc::UnboundedReceiver<SoundManagerMessage>,
    completed_tx: mpsc::UnboundedSender<CalloutCompleted>,
  ) -> Self {
    Self {
      device,
      rx,
      completed_tx,
      sounds: HashMap::new(),
      callout_volume: 0.0,
      music: None,
      callout_queue: CalloutQueue::default(),
      connection: Connection::Waiting {
        retry_at: Instant::now(),
        failures: 0,
      },
    }
  }

  /// Runs until the [`SoundSystem`](crate::SoundSystem) is dropped
  pub async fn run(mut self) {
    self.connect(0);

    let mut tick = tokio::time::interval(TICK);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
      tokio::select! {
        message = self.rx.recv() => match message {
          Some(message) => self.handle(message),
          None => break,
        },
        _ = tick.tick() => self.on_tick(),
      }
    }
  }

  fn handle(&mut self, message: SoundManagerMessage) {
    match message {
      SoundManagerMessage::PreloadPath(name, path) => self.preload(name, path),
      SoundManagerMessage::PreloadEmbedded(name, bytes) => self.preload_embedded(name, bytes),
      SoundManagerMessage::PlaySfx(key) => self.play_sfx(key),
      SoundManagerMessage::PlayCallout(request) => self.play_callout(request),
      SoundManagerMessage::SetCalloutVolume(db) => self.set_callout_volume(db),
      SoundManagerMessage::PlayMusic(path, crossfade) => self.play_music(path, crossfade),
      SoundManagerMessage::StopMusic(crossfade) => self.stop_music(crossfade),
    }
  }

  fn on_tick(&mut self) {
    let now = Instant::now();
    self.update_connection(now);

    for key in self.callout_queue.take_stale(now) {
      report(&self.completed_tx, key, false);
    }

    // check if there are any queued callouts, and if so play the next one instead of un-ducking music
    if let Connection::Connected(output) = &self.connection
      && let Some(active) = &output.active_callout
      && active.handle.state() == PlaybackState::Stopped
    {
      report(&self.completed_tx, active.key, true);
      self.start_next_callout();
    }
  }

  // --- Connection ---

  /// Start an attempt to open the output. It runs off the async runtime, as finding and opening a device blocks, and
  /// the next ticks pick up its result.
  fn connect(&mut self, failures: u32) {
    let (tx, rx) = oneshot::channel();
    let device = self.device;
    tokio::task::spawn_blocking(move || {
      let _ = tx.send(device.open());
    });
    self.connection = Connection::Connecting {
      result: rx,
      failures,
    };
  }

  fn update_connection(&mut self, now: Instant) {
    match &mut self.connection {
      Connection::Connected(output) => {
        if let Err(reason) = output.check_health(now) {
          log::warn!(target: "frontbox_core::sound", "🔇 Lost the audio output ({}), reconnecting", reason);
          self.close_output();
          self.connect(0);
        }
      }
      Connection::Connecting { result, failures } => {
        let failures = *failures;
        match result.try_recv() {
          Ok(Ok(opened)) => self.on_opened(opened, failures, now),
          Ok(Err(reason)) => self.retry_later(failures + 1, reason, now),
          Err(TryRecvError::Empty) => {}
          Err(TryRecvError::Closed) => {
            self.retry_later(failures + 1, "the attempt ended without an answer".into(), now)
          }
        }
      }
      Connection::Waiting { retry_at, failures } => {
        if now >= *retry_at {
          let failures = *failures;
          self.connect(failures);
        }
      }
    }
  }

  fn on_opened(&mut self, opened: Opened, failures: u32, now: Instant) {
    let output = match Output::new(opened.manager, self.callout_volume, now) {
      Ok(output) => output,
      Err(_) => {
        self.retry_later(failures + 1, "kira couldn't add the music and callout tracks".into(), now);
        return;
      }
    };
    if failures == 0 {
      log::info!(target: "frontbox_core::sound", "🔊 Audio output open on '{}'", opened.name);
    } else {
      log::info!(
        target: "frontbox_core::sound",
        "🔊 Audio output open on '{}', after {} failed attempts",
        opened.name,
        failures
      );
    }
    self.connection = Connection::Connected(output);

    if let Some(path) = self.music.clone() {
      self.start_music(&path, Duration::ZERO);
    }
    self.start_next_callout();
  }

  fn retry_later(&mut self, failures: u32, reason: String, now: Instant) {
    let wait = retry_wait(failures);
    // the first failure is worth knowing about, then once in a while so a device that never turns up isn't silent
    if failures == 1 || failures.is_multiple_of(12) {
      log::warn!(
        target: "frontbox_core::sound",
        "🔇 Couldn't open the audio output ({}), attempt {}. Retrying in {:?}",
        reason,
        failures,
        wait
      );
    } else {
      log::debug!(
        target: "frontbox_core::sound",
        "🔇 Couldn't open the audio output ({}), attempt {}. Retrying in {:?}",
        reason,
        failures,
        wait
      );
    }
    self.connection = Connection::Waiting {
      retry_at: now + wait,
      failures,
    };
    // lines can't wait for an output that may be a while, as something may be waiting on them being done with
    self.flush_callouts();
  }

  /// Let go of the output, reporting the line it was saying as not played
  fn close_output(&mut self) {
    let closed = std::mem::replace(
      &mut self.connection,
      Connection::Waiting {
        retry_at: Instant::now(),
        failures: 0,
      },
    );
    if let Connection::Connected(output) = closed {
      if let Some(active) = &output.active_callout {
        report(&self.completed_tx, active.key, false);
      }
      // music that had already played out isn't started again on the next output
      if output.music_played_out() {
        self.music = None;
      }
    }
    self.flush_callouts();
  }

  // --- Messages ---

  fn preload(&mut self, name: &'static str, path: impl AsRef<Path>) {
    log::debug!(target: "frontbox_core::sound", "Preloading sound '{}' at {:?}", name, path.as_ref());
    match StaticSoundData::from_file(path.as_ref()) {
      Ok(sound) => {
        self.sounds.insert(name, sound);
      }
      Err(e) => log::error!(
        target: "frontbox_core::sound",
        "Failed to preload sound {}: {:?}",
        path.as_ref().display(),
        e
      ),
    }
  }

  fn preload_embedded(&mut self, name: &'static str, bytes: &'static [u8]) {
    log::debug!(
      target: "frontbox_core::sound",
      "Preloading embedded sound '{}' ({} bytes)",
      name,
      bytes.len()
    );
    match StaticSoundData::from_cursor(Cursor::new(bytes)) {
      Ok(sound) => {
        self.sounds.insert(name, sound);
      }
      Err(e) => {
        log::error!(target: "frontbox_core::sound", "Failed to preload embedded sound {}: {:?}", name, e)
      }
    }
  }

  fn play_sfx(&mut self, key: &'static str) {
    let Some(sound) = self.sounds.get(key) else {
      log::error!(target: "frontbox_core::sound", "🔊 Sound with key '{}' not found", key);
      return;
    };
    let Connection::Connected(output) = &mut self.connection else {
      log::debug!(target: "frontbox_core::sound", "🔇 No audio output, skipped SFX '{}'", key);
      return;
    };
    output.play_sfx(key, sound.clone());
  }

  fn play_callout(&mut self, request: CalloutRequest) {
    // lines wait while an attempt is under way, as the output is likely moments off, but not once one has failed
    if let Connection::Waiting { .. } = self.connection {
      log::debug!(target: "frontbox_core::sound", "🔇 No audio output, skipped callout '{}'", request.key());
      report(&self.completed_tx, request.key(), false);
      return;
    }
    for key in self.callout_queue.push(request, Instant::now()) {
      report(&self.completed_tx, key, false);
    }
    if let Connection::Connected(output) = &self.connection
      && output.active_callout.is_none()
    {
      self.start_next_callout();
    }
  }

  /// Say the next line that can be said, or give the music its volume back if there's none
  fn start_next_callout(&mut self) {
    let Connection::Connected(output) = &mut self.connection else {
      return;
    };
    let previous_lead_in = output
      .active_callout
      .as_ref()
      .and_then(|active| active.lead_in);
    while let Some(request) = self.callout_queue.pop() {
      let follows_on = previous_lead_in.is_some() && previous_lead_in == request.lead_in_key();
      if let Some(handle) = output.start_callout(&self.sounds, &request, follows_on) {
        output.duck_music();
        output.active_callout = Some(ActiveCallout {
          key: request.key(),
          lead_in: request.lead_in_key(),
          handle,
        });
        return;
      }
      report(&self.completed_tx, request.key(), false);
    }
    output.active_callout = None;
    output.unduck_music();
  }

  /// Report every waiting line as not played
  fn flush_callouts(&mut self) {
    while let Some(request) = self.callout_queue.pop() {
      report(&self.completed_tx, request.key(), false);
    }
  }

  fn set_callout_volume(&mut self, db: f32) {
    self.callout_volume = db;
    if let Connection::Connected(output) = &mut self.connection {
      output.set_callout_volume(db);
    }
  }

  fn play_music(&mut self, path: PathBuf, crossfade: Duration) {
    if let Connection::Connected(_) = self.connection {
      self.start_music(&path, crossfade);
    } else {
      log::info!(
        target: "frontbox_core::sound",
        "🎵 No audio output yet, music {} will start once there is one",
        path.display()
      );
      self.music = Some(path);
    }
  }

  /// Start music on the open output, remembering it if it plays
  fn start_music(&mut self, path: &Path, crossfade: Duration) {
    let Connection::Connected(output) = &mut self.connection else {
      return;
    };
    self.music = output
      .start_music(path, crossfade)
      .then(|| path.to_path_buf());
  }

  fn stop_music(&mut self, crossfade: Duration) {
    let had_music = self.music.take().is_some();
    let stopped = match &mut self.connection {
      Connection::Connected(output) => output.stop_music(crossfade),
      _ => false,
    };
    if stopped {
      log::info!(target: "frontbox_core::sound", "🎵 Stopping music");
    } else if !had_music {
      log::info!(target: "frontbox_core::sound", "Stop music requested, but no music track was active");
    }
  }
}

fn report(completed_tx: &mpsc::UnboundedSender<CalloutCompleted>, key: &'static str, played: bool) {
  let _ = completed_tx.send(CalloutCompleted { key, played });
}
