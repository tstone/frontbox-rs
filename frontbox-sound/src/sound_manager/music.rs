use std::path::{Path, PathBuf};

use frontbox_core::prelude::*;
use kira::clock::{ClockHandle, ClockTime};
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::TrackHandle;
use kira::{Decibels, Easing, PlaySoundError, StartTime, Tween};

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
    let mut data = sound
      .clone()
      .start_time(clock_time(clock, at))
      .start_position(from);
    if crossfade == 0.0 {
      data = data.loop_region(..);
    }
    let handle = match fade_in.filter(|fade_in| !fade_in.is_zero()) {
      Some(fade_in) => play_fading_in(track, data, clock_time(clock, at), fade_in)?,
      None => track.play(data)?,
    };

    let mut music = Self {
      path: path.to_path_buf(),
      sound,
      crossfade,
      playing: Pass {
        handle,
        start: at,
        from,
      },
      next: None,
    };
    music.schedule_next(track, clock);
    Ok(music)
  }

  /// Move on to the next pass once it's started, and schedule the one after
  pub fn update(&mut self, track: &mut TrackHandle, clock: &ClockHandle) {
    if self
      .next
      .as_ref()
      .is_some_and(|next| clock_seconds(clock) >= next.start)
    {
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
    self.stop(Tween {
      duration,
      easing: FADE_OUT_EASING,
      ..Default::default()
    });
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
      .start_position(from);
    match play_fading_in(track, data, clock_time(clock, at), fade) {
      Ok(handle) => {
        self.playing.handle.stop(Tween {
          start_time: clock_time(clock, at),
          duration: fade,
          easing: FADE_OUT_EASING,
        });
        self.next = Some(Pass {
          handle,
          start: at,
          from,
        });
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

/// Play `data` silent, fading in from `at` on the clock (a fade-in runs from when the sound is played, not when it
/// starts). The fade-in is on the sound's volume rather than kira's `fade_in_tween`: that shares a parameter with
/// `stop`, so the fade-out scheduled for the pass's end would replace the fade-in, leaving the pass silent.
fn play_fading_in(
  track: &mut TrackHandle,
  data: StaticSoundData,
  at: StartTime,
  fade: Duration,
) -> Result<StaticSoundHandle, PlaySoundError<()>> {
  let mut handle = track.play(data.volume(Decibels::SILENCE))?;
  handle.set_volume(
    Decibels::IDENTITY,
    Tween {
      start_time: at,
      duration: fade,
      easing: FADE_IN_EASING,
    },
  );
  Ok(handle)
}

/// The output's clock ticks once a second, so its time in ticks is in seconds
fn clock_seconds(clock: &ClockHandle) -> f64 {
  let time = clock.time();
  time.ticks as f64 + time.fraction
}

fn clock_time(clock: &ClockHandle, seconds: f64) -> StartTime {
  StartTime::ClockTime(ClockTime::from_ticks_f64(clock.id(), seconds))
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Mutex};

  use kira::backend::mock::{MockBackend, MockBackendSettings};
  use kira::clock::ClockSpeed;
  use kira::effect::{Effect, EffectBuilder};
  use kira::info::Info;
  use kira::track::TrackBuilder;
  use kira::{AudioManager, AudioManagerSettings, Frame};

  use super::*;

  const SAMPLE_RATE: u32 = 1000;

  /// Records the loudest sample of each buffer passing through the track
  struct Peaks(Arc<Mutex<Vec<f32>>>);

  impl EffectBuilder for Peaks {
    type Handle = ();

    fn build(self) -> (Box<dyn Effect>, ()) {
      (Box::new(self), ())
    }
  }

  impl Effect for Peaks {
    fn process(&mut self, input: &mut [Frame], _dt: f64, _info: &Info) {
      let peak = input
        .iter()
        .map(|frame| frame.left.abs())
        .fold(0.0, f32::max);
      self.0.lock().unwrap().push(peak);
    }
  }

  struct Output {
    manager: AudioManager<MockBackend>,
    track: TrackHandle,
    clock: ClockHandle,
    peaks: Arc<Mutex<Vec<f32>>>,
  }

  impl Output {
    fn new() -> Self {
      let mut manager = AudioManager::<MockBackend>::new(AudioManagerSettings {
        backend_settings: MockBackendSettings {
          sample_rate: SAMPLE_RATE,
        },
        ..Default::default()
      })
      .unwrap();
      let peaks = Arc::new(Mutex::new(Vec::new()));
      let track = manager
        .add_sub_track(TrackBuilder::new().with_effect(Peaks(peaks.clone())))
        .unwrap();
      let mut clock = manager.add_clock(ClockSpeed::TicksPerSecond(1.0)).unwrap();
      clock.start();
      let mut output = Self {
        manager,
        track,
        clock,
        peaks,
      };
      // the clock starts on the audio thread
      output.render(None, 0.2);
      output
    }

    /// Render `seconds` of audio, updating `music` between buffers as the output does. Returns the loudest sample.
    fn render(&mut self, mut music: Option<&mut Music>, seconds: f64) -> f32 {
      self.peaks.lock().unwrap().clear();
      let buffers = (seconds * SAMPLE_RATE as f64 / 128.0).ceil() as usize;
      for _ in 0..buffers {
        let backend = self.manager.backend_mut();
        backend.on_start_processing();
        backend.process();
        if let Some(music) = music.as_deref_mut() {
          music.update(&mut self.track, &self.clock);
        }
      }
      self
        .peaks
        .lock()
        .unwrap()
        .iter()
        .copied()
        .fold(0.0, f32::max)
    }

    /// The quietest buffer in `seconds` of audio
    fn quietest(&mut self, music: &mut Music, seconds: f64) -> f32 {
      self.render(Some(music), seconds);
      self
        .peaks
        .lock()
        .unwrap()
        .iter()
        .copied()
        .fold(f32::MAX, f32::min)
    }

    fn start(&mut self, fade_in: Option<Duration>) -> Music {
      // three seconds at full volume
      let sound = StaticSoundData {
        sample_rate: SAMPLE_RATE,
        frames: vec![Frame::from_mono(1.0); 3 * SAMPLE_RATE as usize].into(),
        settings: Default::default(),
        slice: None,
      };
      Music::start(
        &mut self.track,
        &self.clock,
        Path::new("music.wav"),
        sound,
        Duration::from_millis(500),
        0.0,
        fade_in,
      )
      .unwrap()
    }
  }

  #[test]
  fn music_crossfaded_into_is_heard_once_its_fade_in_is_over() {
    let mut output = Output::new();
    let mut music = output.start(Some(Duration::from_millis(500)));
    output.render(Some(&mut music), 0.7);
    assert!(output.quietest(&mut music, 1.0) > 0.9);
  }

  #[test]
  fn music_is_heard_through_each_loop() {
    let mut output = Output::new();
    let mut music = output.start(None);
    // through two loops, each pass back at full volume once the crossfade into it is over
    assert!(output.quietest(&mut music, 7.0) > 0.95);
  }
}
