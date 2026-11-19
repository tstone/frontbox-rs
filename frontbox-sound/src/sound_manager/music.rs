use std::path::{Path, PathBuf};

use frontbox_core::prelude::*;
use kira::clock::{ClockHandle, ClockTime};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::TrackHandle;
use kira::{Easing, PlaySoundError, StartTime, Tween};

/// How far ahead of the audio clock a track's first pass is scheduled, so every pass is timed on the clock rather than
/// from whenever the command reaches the audio thread
const START_LEAD: f64 = 0.05;
/// The easing on each side of a crossfade. Kira fades in decibels, so these keep the two passes' combined power within
/// about 1dB of constant, where an unshaped fade dips in the middle.
const FADE_IN_EASING: Easing = Easing::OutPowi(4);
const FADE_OUT_EASING: Easing = Easing::InPowi(4);

/// A music track that plays over and over until it's stopped. Without a crossfade it loops start to end with no gap.
/// With one, each pass is a separate sound: the next starts `crossfade` into the track while the last `crossfade` of
/// the one before plays, and they crossfade. The track's first `crossfade` only plays on the first pass.
pub(super) struct Music {
  path: PathBuf,
  sound: StaticSoundData,
  crossfade: f64,
  /// The pass that started most recently
  playing: Pass,
  /// The pass scheduled to crossfade in after `playing`
  next: Option<Pass>,
}

impl Music {
  /// Start playing `sound` from `from` seconds in, fading in by `fade_in` if given
  pub fn start(
    track: &mut TrackHandle,
    clock: &ClockHandle,
    path: &Path,
    sound: StaticSoundData,
    crossfade: Duration,
    from: f64,
    fade_in: Option<Duration>,
  ) -> Result<Self, PlaySoundError<()>> {
    let length = sound.duration().as_secs_f64();
    // passes more than crossfade apart, so no more than two ever play at once
    let crossfade = crossfade.as_secs_f64().min(length / 3.0);
    // picked up partway into the last crossfade, there'd be too little left to fade out over
    let from = from.min(length - crossfade);
    let at = clock_seconds(clock) + START_LEAD;
    let mut data = sound.clone().start_time(clock_time(clock, at)).start_position(from);
    if crossfade == 0.0 {
      data = data.loop_region(..);
    }
    if let Some(fade_in) = fade_in.filter(|fade_in| !fade_in.is_zero()) {
      data = data.fade_in_tween(Tween {
        start_time: clock_time(clock, at),
        duration: fade_in,
        easing: FADE_IN_EASING,
      });
    }
    let handle = track.play(data)?;

    let mut music = Self {
      path: path.to_path_buf(),
      sound,
      crossfade,
      playing: Pass { handle, start: at, from },
      next: None,
    };
    music.schedule_next(track, clock);
    Ok(music)
  }

  /// Move on to the next pass once it's started, and schedule the one after
  pub fn update(&mut self, track: &mut TrackHandle, clock: &ClockHandle) {
    if self.next.as_ref().is_some_and(|next| clock_seconds(clock) >= next.start) {
      self.playing = self.next.take().unwrap();
      self.schedule_next(track, clock);
    }
  }

  /// Fade out every pass, including one waiting to start
  pub fn stop(&mut self, tween: Tween) {
    self.playing.handle.stop(tween);
    if let Some(next) = &mut self.next {
      next.handle.stop(tween);
    }
  }

  /// Fade out for a crossfade into other music
  pub fn crossfade_out(&mut self, duration: Duration) {
    self.stop(Tween { duration, easing: FADE_OUT_EASING, ..Default::default() });
  }

  /// How far into the track the latest pass is, in seconds. Read from the clock, which a lost device leaves where it
  /// stopped, so it says where the music was cut off.
  pub fn position(&self, clock: &ClockHandle) -> f64 {
    let now = clock_seconds(clock);
    let pass = match &self.next {
      Some(next) if now >= next.start => next,
      _ => &self.playing,
    };
    let length = self.sound.duration().as_secs_f64();
    (pass.from + (now - pass.start).max(0.0)) % length
  }

  /// Schedule the pass after the one playing to start as its last `crossfade` begins, and the playing one to fade out
  /// as it does
  fn schedule_next(&mut self, track: &mut TrackHandle, clock: &ClockHandle) {
    if self.crossfade == 0.0 {
      return;
    }
    let length = self.sound.duration().as_secs_f64();
    let from = self.crossfade;
    let at = self.playing.start + length - self.playing.from - self.crossfade;
    let fade = Duration::from_secs_f64(self.crossfade);
    let data = self
      .sound
      .clone()
      .start_time(clock_time(clock, at))
      .start_position(from)
      // timed on the clock, as a fade-in runs from when the sound is played, not when it starts
      .fade_in_tween(Tween { start_time: clock_time(clock, at), duration: fade, easing: FADE_IN_EASING });
    match track.play(data) {
      Ok(handle) => {
        self.playing.handle.stop(Tween {
          start_time: clock_time(clock, at),
          duration: fade,
          easing: FADE_OUT_EASING,
        });
        self.next = Some(Pass { handle, start: at, from });
      }
      Err(e) => log::error!(
        target: "frontbox::sound",
        "🎵 Failed to schedule the next pass of music {}: {:?}",
        self.path.display(),
        e
      ),
    }
  }
}

/// One play of the track
struct Pass {
  handle: StaticSoundHandle,
  /// When it starts, in seconds on the output's clock
  start: f64,
  /// How far into the track it starts, in seconds
  from: f64,
}

/// The output's clock ticks once a second, so its time in ticks is in seconds
fn clock_seconds(clock: &ClockHandle) -> f64 {
  let time = clock.time();
  time.ticks as f64 + time.fraction
}

fn clock_time(clock: &ClockHandle, seconds: f64) -> StartTime {
  StartTime::ClockTime(ClockTime::from_ticks_f64(clock.id(), seconds))
}
