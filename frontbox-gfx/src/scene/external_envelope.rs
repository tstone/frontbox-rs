use std::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::animation::Cycle;
use crate::scene::sound;
use crate::scene::{Animatable, Env, SceneError, Spread, Timing};

/// A value that follows the rise and fall of something outside the scene's graphics, as an envelope follower does
///
/// What it follows is its [`EnvelopeSource`]: content analyzed ahead of time into a curve from 0 to 1, such as a sound's
/// loudness, and where on the scene's timeline that content plays. The curve is slewed, so it rises no faster than
/// `attack` and falls no faster than `release` allow, then moves the value through `spread` as a modulator's wave does.
///
/// Nothing runs while the scene plays. The slewed curve is worked out when the scene's assets load
/// ([`PreparedEnvelope`]), so any moment can be sampled on its own, seeking and stalls included.
///
/// In a scene file:
/// `ExternalEnvelope(source: Sound(layer: "hits"), spread: Float(0.0, 1.0), attack: 0.005, release: 0.15)`
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalEnvelope {
  pub source: EnvelopeSource,
  /// The range it moves through, from where the curve is at 0 to where it's at its peak. For a `Vec2` or a color, every
  /// part moves together.
  pub spread: Spread,
  /// Seconds the curve takes to rise from 0 to its peak, at the fastest. 0 rises as fast as the content does.
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<f32>", optional))]
  pub attack: f32,
  /// Seconds the curve takes to fall from its peak to 0, at the fastest. 0 falls as fast as the content does.
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<f32>", optional))]
  pub release: f32,
}

impl ExternalEnvelope {
  pub fn new(source: EnvelopeSource, spread: Spread) -> Self {
    Self {
      source,
      spread,
      attack: 0.0,
      release: 0.0,
    }
  }

  pub fn attack(mut self, seconds: f32) -> Self {
    self.attack = seconds;
    self
  }

  pub fn release(mut self, seconds: f32) -> Self {
    self.release = seconds;
    self
  }

  /// What it follows and how it's slewed, which is what [`Assets`](crate::scene::Assets) prepares it by
  pub fn key(&self) -> EnvelopeKey {
    EnvelopeKey {
      source: self.source.clone(),
      attack: self.attack.to_bits(),
      release: self.release.to_bits(),
    }
  }

  /// The value at `env`. Without its [`PreparedEnvelope`] in the assets, it gives the type's default.
  pub(crate) fn resolve<T: Animatable>(&self, env: &Env) -> T {
    let Some(prepared) = env.assets.envelope(&self.key()) else {
      return T::default();
    };
    self
      .spread
      .sample([prepared.height(env); 3])
      .unwrap_or_default()
  }
}

/// What an [`ExternalEnvelope`] follows
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EnvelopeSource {
  /// The sound of the scene's sound layer named `layer`, analyzed by `analysis`, as the layer plays it: from its start in
  /// each cycle, skipping what it skips and cut off where it's cut off. Where cycles overlap, it follows whichever is
  /// higher, each slewed on its own.
  Sound {
    layer: String,
    #[serde(default, skip_serializing_if = "Analysis::is_rms")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Analysis>", optional))]
    analysis: Analysis,
  },
}

impl EnvelopeSource {
  /// The sound layer named `layer`, analyzed for its loudness
  pub fn sound(layer: impl Into<String>) -> Self {
    Self::Sound {
      layer: layer.into(),
      analysis: Analysis::Rms,
    }
  }
}

/// How content is turned into a curve from 0 to 1
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(
  Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Analysis {
  /// Loudness: the root mean square of the sound over windows of 20 ms, every 5 ms, scaled so its loudest is 1
  #[default]
  Rms,
}

impl Analysis {
  fn is_rms(&self) -> bool {
    *self == Self::Rms
  }
}

/// An [`ExternalEnvelope`]'s source and slew: what a scene's assets prepare it by, and what
/// [`Inputs::envelopes`](crate::scene::Inputs::envelopes) lists
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EnvelopeKey {
  pub source: EnvelopeSource,
  /// As bits, so keys can be compared and hashed
  attack: u32,
  release: u32,
}

impl EnvelopeKey {
  pub fn attack(&self) -> f32 {
    f32::from_bits(self.attack)
  }

  pub fn release(&self) -> f32 {
    f32::from_bits(self.release)
  }

  /// Errors for an attack or release that isn't 0 or more seconds
  pub fn check(&self) -> Result<(), SceneError> {
    let seconds = |slew: f32| slew.is_finite() && slew >= 0.0;
    if seconds(self.attack()) && seconds(self.release()) {
      return Ok(());
    }
    let EnvelopeSource::Sound { layer, .. } = &self.source;
    Err(SceneError::NegativeSlew {
      sound: layer.clone(),
    })
  }
}

/// Content analyzed into a curve, before any slew: values from 0 to 1, `rate` of them a second, from the start of the
/// content. From [`Self::analyze`].
#[derive(Clone, PartialEq)]
pub struct EnvelopeContent {
  rate: f32,
  values: Vec<f32>,
}

impl EnvelopeContent {
  /// `values` from the start of the content, `rate` of them a second. Each is clamped to 0 to 1.
  pub fn new(rate: f32, values: Vec<f32>) -> Self {
    Self {
      rate,
      values: values
        .into_iter()
        .map(|value| value.clamp(0.0, 1.0))
        .collect(),
    }
  }

  /// A sound file's contents analyzed by `analysis`, for an external envelope to follow. `name` is the file's, whose
  /// extension hints at its format. The error says why it couldn't be decoded. Decodes the whole sound, so don't call
  /// this mid-frame.
  pub fn analyze(name: &str, bytes: Vec<u8>, analysis: Analysis) -> Result<Self, String> {
    let (power, sample_rate) = sound::decode_power(name, bytes)?;
    Ok(match analysis {
      Analysis::Rms => Self::rms(&power, sample_rate),
    })
  }

  /// [`Analysis::Rms`] of a sound, given the mean square of its channels at each frame, `sample_rate` frames a second
  pub(crate) fn rms(power: &[f32], sample_rate: u32) -> Self {
    let frames_per_value = (sample_rate as f32 * RMS_HOP).round().max(1.0) as usize;
    let half_window = (sample_rate as f32 * RMS_WINDOW / 2.0).round().max(1.0) as usize;
    // running sums, so each window's mean is a subtraction
    let mut sums = Vec::with_capacity(power.len() + 1);
    sums.push(0.0f64);
    for frame in power {
      sums.push(sums.last().unwrap() + *frame as f64);
    }
    let mut values: Vec<f32> = (0..power.len().div_ceil(frames_per_value))
      .map(|index| {
        let center = index * frames_per_value;
        let (from, to) = (
          center.saturating_sub(half_window),
          (center + half_window).min(power.len()),
        );
        ((sums[to] - sums[from]) / (to - from) as f64).sqrt() as f32
      })
      .collect();
    let peak = values.iter().copied().fold(0.0, f32::max);
    if peak > 0.0 {
      values.iter_mut().for_each(|value| *value /= peak);
    }
    Self::new(sample_rate as f32 / frames_per_value as f32, values)
  }

  /// Values a second
  pub fn rate(&self) -> f32 {
    self.rate
  }

  pub fn values(&self) -> &[f32] {
    &self.values
  }
}

impl fmt::Debug for EnvelopeContent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("EnvelopeContent")
      .field("rate", &self.rate)
      .field("values", &self.values.len())
      .finish()
  }
}

/// Seconds between [`Analysis::Rms`] values
const RMS_HOP: f32 = 0.005;

/// Seconds of sound each [`Analysis::Rms`] value is taken over: long enough to hold a whole wave of a low note, so a bass
/// doesn't flicker
const RMS_WINDOW: f32 = 0.02;

/// The most cycles an envelope follows at once, where a sound is longer than several cycles. The oldest are left out.
const MAX_OVERLAPS: u32 = 32;

/// An [`ExternalEnvelope`] ready to sample: its content slewed, and where that content plays in each cycle
///
/// Slewing depends on everything before, so it's done here, once, when the scene's assets load. Sampling is then a
/// lookup at any moment.
#[derive(Clone)]
pub struct PreparedEnvelope {
  rate: f32,
  /// Slewed, from the start of the content, then falling to 0 at the release's pace past its end
  values: Arc<[f32]>,
  release: f32,
  placement: Placement,
}

impl PreparedEnvelope {
  pub fn new(content: &EnvelopeContent, placement: Placement, attack: f32, release: f32) -> Self {
    // a slew that isn't more than 0, including one that fails `EnvelopeKey::check`, is none
    let most = |slew: f32| match slew > 0.0 {
      true => 1.0 / (content.rate * slew),
      false => f32::INFINITY,
    };
    let (rise, fall) = (most(attack), most(release));
    let mut values = Vec::with_capacity(content.values.len());
    let mut at = 0.0f32;
    for value in &content.values {
      at = value.clamp(at - fall, at + rise);
      values.push(at);
    }
    while at > 0.0 {
      at = (at - fall).max(0.0);
      values.push(at);
    }
    Self {
      rate: content.rate,
      values: values.into(),
      release,
      placement,
    }
  }

  /// The slewed curve's height at `env`, 0 to 1: the highest of the plays of its content still going
  fn height(&self, env: &Env) -> f32 {
    let Some(timing) = env.timing else {
      return self.play(env.time - self.placement.start);
    };
    let now = env.cycle as f32 * timing.duration + env.time;
    self
      .plays(timing, now)
      .map(|start| self.play(now - start))
      .fold(0.0, f32::max)
  }

  /// When each play still going at `now` started, in seconds since the scene started
  fn plays(&self, timing: Timing, now: f32) -> impl Iterator<Item = f32> {
    let Placement { start, .. } = self.placement;
    let cycles = match timing.cycle {
      Cycle::Once => 1,
      Cycle::Times(times) => times,
      Cycle::Forever => u32::MAX,
    };
    let since_first = now - start;
    let last = (since_first / timing.duration)
      .floor()
      .min(cycles.saturating_sub(1) as f32);
    let first = ((since_first - self.length()) / timing.duration)
      .ceil()
      .max(0.0)
      .max(last - (MAX_OVERLAPS - 1) as f32);
    let plays = if last < first {
      0
    } else {
      (last - first) as u32 + 1
    };
    (0..plays).map(move |index| (first + index as f32) * timing.duration + start)
  }

  /// Seconds a play stays above 0 at most
  fn length(&self) -> f32 {
    let Placement { start, skip, end } = self.placement;
    let content = self.values.len() as f32 / self.rate - skip;
    end.map_or(content, |end| content.min(end - start + self.release))
  }

  /// The height `since` seconds into a play
  fn play(&self, since: f32) -> f32 {
    let Placement { start, skip, end } = self.placement;
    match end.map(|end| end - start) {
      Some(cut) if since >= cut => {
        let fallen = match self.release > 0.0 {
          true => (since - cut) / self.release,
          false => f32::INFINITY,
        };
        (self.at(cut + skip) - fallen).max(0.0)
      }
      _ if since < 0.0 => 0.0,
      _ => self.at(since + skip),
    }
  }

  /// The slewed curve `seconds` into the content, between its values
  fn at(&self, seconds: f32) -> f32 {
    let position = seconds * self.rate;
    let index = position.floor() as usize;
    let Some(&before) = self.values.get(index) else {
      return 0.0;
    };
    let after = self.values.get(index + 1).copied().unwrap_or(0.0);
    before + (after - before) * position.fract()
  }
}

impl fmt::Debug for PreparedEnvelope {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("PreparedEnvelope")
      .field("rate", &self.rate)
      .field("values", &self.values.len())
      .field("release", &self.release)
      .field("placement", &self.placement)
      .finish()
  }
}

/// Where content plays in each cycle of a scene, in seconds, as a scene's sound layer says: from `start` into the cycle,
/// `skip` seconds into the content, and cut off at `end` into the cycle if it has one
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
  pub start: f32,
  pub skip: f32,
  pub end: Option<f32>,
}

fn is_zero(seconds: &f32) -> bool {
  *seconds == 0.0
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Assets, Prop, Variables, file};

  const AT_ONCE: Placement = Placement {
    start: 0.0,
    skip: 0.0,
    end: None,
  };

  /// Ten values a second, so each is a tenth of a second
  fn content(values: &[f32]) -> EnvelopeContent {
    EnvelopeContent::new(10.0, values.to_vec())
  }

  fn heights(prepared: &PreparedEnvelope, timing: Timing, times: &[f32]) -> Vec<f32> {
    let variables = Variables::new();
    times
      .iter()
      .map(|&time| {
        let env = Env {
          time: timing.local_time(time),
          cycle: timing.cycle_index(time),
          timing: Some(timing),
          ..env(0.0, &variables)
        };
        (prepared.height(&env) * 100.0).round() / 100.0
      })
      .collect()
  }

  fn forever(duration: f32) -> Timing {
    Timing::new(duration, Cycle::Forever)
  }

  #[test]
  fn without_slew_it_follows_the_content_between_its_values() {
    let prepared = PreparedEnvelope::new(&content(&[0.0, 1.0, 0.5]), AT_ONCE, 0.0, 0.0);
    assert_eq!(
      heights(&prepared, forever(10.0), &[0.0, 0.05, 0.1, 0.2, 0.3, 1.0]),
      [0.0, 0.5, 1.0, 0.5, 0.0, 0.0]
    );
  }

  #[test]
  fn attack_and_release_limit_how_fast_it_moves() {
    // rising 0.5 and falling 0.25 at most each tenth of a second
    let prepared = PreparedEnvelope::new(&content(&[1.0, 1.0, 1.0, 0.0]), AT_ONCE, 0.2, 0.4);
    assert_eq!(
      heights(
        &prepared,
        forever(10.0),
        &[0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]
      ),
      [0.5, 1.0, 1.0, 0.75, 0.5, 0.25, 0.0, 0.0],
      "it releases past the end of its content"
    );
  }

  #[test]
  fn it_plays_from_the_layers_start_skipping_what_it_skips() {
    let placement = Placement {
      start: 0.5,
      skip: 0.1,
      end: None,
    };
    let prepared = PreparedEnvelope::new(&content(&[0.0, 1.0, 0.5]), placement, 0.0, 0.0);
    assert_eq!(
      heights(&prepared, forever(10.0), &[0.4, 0.5, 0.6, 0.7]),
      [0.0, 1.0, 0.5, 0.0]
    );
  }

  #[test]
  fn a_cut_off_falls_at_the_release_pace() {
    let held = content(&[1.0; 20]);
    let cut = Placement {
      end: Some(0.5),
      ..AT_ONCE
    };
    let released = PreparedEnvelope::new(&held, cut, 0.0, 0.4);
    assert_eq!(
      heights(&released, forever(10.0), &[0.4, 0.5, 0.7, 0.9, 1.0]),
      [1.0, 1.0, 0.5, 0.0, 0.0]
    );
    let sudden = PreparedEnvelope::new(&held, cut, 0.0, 0.0);
    assert_eq!(heights(&sudden, forever(10.0), &[0.4, 0.5]), [1.0, 0.0]);
  }

  #[test]
  fn each_cycle_plays_it_again() {
    let prepared = PreparedEnvelope::new(&content(&[1.0, 0.0]), AT_ONCE, 0.0, 0.0);
    assert_eq!(
      heights(&prepared, forever(1.0), &[0.0, 0.5, 1.0, 1.5, 7.0]),
      [1.0, 0.0, 1.0, 0.0, 1.0]
    );
    let twice = Timing::new(1.0, Cycle::Times(2));
    assert_eq!(
      heights(&prepared, twice, &[1.0, 2.0, 5.0]),
      [1.0, 0.0, 0.0],
      "no third cycle"
    );
  }

  #[test]
  fn where_cycles_overlap_it_follows_the_higher() {
    // half a second of sound, a cycle every fifth of a second: each new play starts low, under the last one's peak
    let swell = content(&[0.2, 0.6, 1.0, 1.0, 0.0]);
    let prepared = PreparedEnvelope::new(&swell, AT_ONCE, 0.0, 0.0);
    assert_eq!(
      heights(&prepared, forever(0.2), &[0.1, 0.2, 0.3]),
      [0.6, 1.0, 1.0]
    );
  }

  #[test]
  fn rms_is_loudness_scaled_so_the_loudest_is_one() {
    // a second at 1000 frames a second: silent, then loud, then half as loud
    let power: Vec<f32> = (0..1000)
      .map(|frame| match frame {
        0..300 => 0.0,
        300..600 => 0.16,
        _ => 0.04,
      })
      .collect();
    let analyzed = EnvelopeContent::rms(&power, 1000);
    assert_eq!(analyzed.rate(), 200.0);
    let at =
      |seconds: f32| (analyzed.values()[(seconds * 200.0) as usize] * 1000.0).round() / 1000.0;
    assert_eq!([at(0.1), at(0.45), at(0.8)], [0.0, 1.0, 0.5]);
    assert_eq!(
      EnvelopeContent::rms(&[0.0; 100], 1000).values(),
      [0.0; 20],
      "silence stays at 0"
    );
  }

  #[test]
  fn it_moves_through_its_spread_and_rests_without_its_assets() {
    let envelope = ExternalEnvelope::new(EnvelopeSource::sound("hits"), Spread::Float(10.0, 20.0));
    let mut assets = Assets::new();
    assets.insert_envelope(
      envelope.key(),
      PreparedEnvelope::new(&content(&[0.5]), AT_ONCE, 0.0, 0.0),
    );
    let variables = Variables::new();
    let timing = Some(forever(1.0));
    let with = Env {
      assets: &assets,
      timing,
      ..env(0.0, &variables)
    };
    let without = Env {
      timing,
      ..env(0.0, &variables)
    };
    assert_eq!(
      Prop::<f32>::ExternalEnvelope(envelope.clone()).resolve(&with),
      15.0
    );
    assert_eq!(
      Prop::<f32>::ExternalEnvelope(envelope).resolve(&without),
      0.0
    );
  }

  #[test]
  fn a_slew_less_than_none_fails_the_check() {
    let envelope = ExternalEnvelope::new(EnvelopeSource::sound("hits"), Spread::Float(0.0, 1.0));
    assert!(envelope.clone().attack(0.1).key().check().is_ok());
    assert_eq!(
      envelope.clone().release(-0.1).key().check(),
      Err(SceneError::NegativeSlew {
        sound: "hits".into()
      })
    );
    assert!(envelope.attack(f32::NAN).key().check().is_err());
  }

  #[test]
  fn it_round_trips_through_ron_with_defaults_left_out() {
    let plain = ExternalEnvelope::new(EnvelopeSource::sound("hits"), Spread::Float(0.0, 1.0));
    let slewed = plain.clone().attack(0.01).release(0.2);
    for envelope in [plain, slewed] {
      let prop = Prop::<f32>::ExternalEnvelope(envelope.clone());
      let written = file::save(&prop).unwrap();
      let Prop::ExternalEnvelope(read) = file::load::<Prop<f32>>(&written).unwrap() else {
        panic!("{written}");
      };
      assert_eq!(read, envelope);
      assert!(!written.contains("analysis"), "{written}");
      assert_eq!(
        written.contains("attack"),
        envelope.attack != 0.0,
        "{written}"
      );
    }
  }
}
