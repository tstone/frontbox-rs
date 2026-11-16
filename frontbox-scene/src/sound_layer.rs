use frontbox_gfx::animation::Cycle;
use frontbox_gfx::scene::{LayerFrame, Prop, SceneError, Timing};
use serde::{Deserialize, Serialize};

/// A sound played once in each cycle of a scene. `SoundSystem` plays it when the playback's clock passes its start, and
/// once started it plays to its end, or to `end` if that comes first, even if the playback stops.
///
/// In RON: `Sound(sound: "sounds/zap.mp3", volume: Fixed(1.0), start: 0.5, skip: 0.1, end: 1.2)`. With no `start`, it
/// plays at the start of the cycle, and with no `skip`, from the beginning of the sound.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoundLayer {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub name: Option<String>,
  /// The sound, by its name in the content folder
  pub sound: String,
  /// How loud it plays, taken when it starts: 1.0 is as recorded, 0.0 silent
  #[serde(default = "full_volume")]
  #[cfg_attr(feature = "ts", ts(as = "Option<Prop<f32>>", optional))]
  pub volume: Prop<f32>,
  /// Seconds into each cycle that it starts
  #[serde(default)]
  pub start: f32,
  /// Seconds cut from the beginning of the sound, so it plays from that far in
  #[serde(default, skip_serializing_if = "is_zero")]
  #[cfg_attr(feature = "ts", ts(as = "Option<f32>", optional))]
  pub skip: f32,
  /// Seconds into each cycle that it's cut off, if that's before it ends. It can be past the end of the cycle, as a sound
  /// plays on past it.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub end: Option<f32>,
}

impl SoundLayer {
  /// `sound` played at the start
  pub fn new(sound: impl Into<String>) -> Self {
    Self { name: None, sound: sound.into(), volume: full_volume(), start: 0.0, skip: 0.0, end: None }
  }

  pub fn name(mut self, name: impl Into<String>) -> Self {
    self.name = Some(name.into());
    self
  }

  pub fn volume(mut self, volume: impl Into<Prop<f32>>) -> Self {
    self.volume = volume.into();
    self
  }

  /// Seconds into each cycle that it starts
  pub fn start(mut self, start: f32) -> Self {
    self.start = start;
    self
  }

  /// Seconds cut from the beginning of the sound
  pub fn skip(mut self, skip: f32) -> Self {
    self.skip = skip;
    self
  }

  /// Seconds into each cycle that it's cut off
  pub fn end(mut self, end: f32) -> Self {
    self.end = Some(end);
    self
  }

  /// How logs and errors refer to the layer at `index`, as [`Layer::label`](frontbox_gfx::scene::Layer::label) does
  pub fn label(&self, index: usize) -> String {
    match &self.name {
      Some(name) => format!("layer \"{name}\""),
      None => format!("layer {}", index + 1),
    }
  }

  /// Errors for a start outside a cycle, an end before it, or less than nothing skipped, and for a volume or a start
  /// after the top in a static scene
  pub fn check(&self, index: usize, timing: Option<Timing>) -> Result<(), SceneError> {
    if timing.is_none() && (self.volume.is_animated() || self.start != 0.0) {
      return Err(SceneError::NoTiming { layer: self.label(index) });
    }
    if !(self.skip >= 0.0) {
      return Err(SceneError::NegativeSkip { layer: self.label(index) });
    }
    let duration = timing.map_or(f32::INFINITY, |timing| timing.duration);
    if !(self.start >= 0.0 && self.start < duration) {
      return Err(SceneError::StartOutsideCycle { layer: self.label(index) });
    }
    match self.end {
      Some(end) if !(end > self.start) => Err(SceneError::EndBeforeStart { layer: self.label(index) }),
      _ => Ok(()),
    }
  }

  /// When it starts after `after` and up to `until`, in seconds since the scene started. `None` is from the top, taking
  /// in a start right at the top. If it's passed more than once in that time, as a long stall can pass several cycles,
  /// it's only given at its last.
  pub fn started(&self, timing: Option<Timing>, after: Option<f32>, until: f32) -> Option<f32> {
    last_start(self.start, timing, after.unwrap_or(f32::NEG_INFINITY), until)
  }

  /// Seconds it plays before it's cut off, or `None` to play to its end
  pub fn length(&self) -> Option<f32> {
    self.end.map(|end| end - self.start)
  }

  /// How loud it plays starting in `frame`
  pub fn volume_at(&self, frame: &LayerFrame) -> f32 {
    self.volume.at(frame).max(0.0)
  }
}

/// A sound starting, from [`Scene::sounds_started`](crate::Scene::sounds_started)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoundStart<'a> {
  /// The sound, by its name in the content folder
  pub sound: &'a str,
  /// Seconds since the scene started
  pub time: f32,
  /// 1.0 is as recorded
  pub volume: f32,
  /// Seconds cut from the beginning of the sound
  pub skip: f32,
  /// Seconds it plays before it's cut off, or `None` to play to its end
  pub length: Option<f32>,
}

fn full_volume() -> Prop<f32> {
  Prop::Fixed(1.0)
}

fn is_zero(seconds: &f32) -> bool {
  *seconds == 0.0
}

/// The last time after `after` and up to `until` that a start `at` seconds into a cycle comes
fn last_start(at: f32, timing: Option<Timing>, after: f32, until: f32) -> Option<f32> {
  let Some(timing) = timing else {
    return (after < at && at <= until).then_some(at);
  };
  let cycles = match timing.cycle {
    Cycle::Once => 1,
    Cycle::Times(times) => times,
    Cycle::Forever => u32::MAX,
  };
  // the last cycle whose start is by `until`
  let last = ((until - at) / timing.duration).floor();
  if last < 0.0 {
    return None;
  }
  let start = last.min(cycles as f32 - 1.0) * timing.duration + at;
  (after < start && start <= until).then_some(start)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn layer() -> SoundLayer {
    SoundLayer::new("a.wav").start(0.4)
  }

  #[test]
  fn it_starts_once_the_clock_passes_its_start() {
    let (layer, timing) = (layer(), Some(Timing::new(1.0, Cycle::Once)));
    assert_eq!(layer.started(timing, None, 0.3), None);
    assert_eq!(layer.started(timing, Some(0.3), 0.4), Some(0.4));
    assert_eq!(layer.started(timing, Some(0.4), 2.0), None);
    assert_eq!(SoundLayer::new("a.wav").started(timing, None, 0.0), Some(0.0), "right at the top");
  }

  #[test]
  fn each_cycle_starts_it_again() {
    let (layer, timing) = (layer(), Some(Timing::new(1.0, Cycle::Times(2))));
    assert_eq!(layer.started(timing, Some(1.0), 1.5), Some(1.4));
    assert_eq!(layer.started(timing, Some(1.5), 2.5), None, "no third cycle");
  }

  #[test]
  fn a_stall_plays_what_it_passed_once() {
    let (layer, timing) = (layer(), Some(Timing::new(1.0, Cycle::Forever)));
    assert_eq!(layer.started(timing, Some(0.5), 3.2), Some(2.4));
  }

  #[test]
  fn a_static_scene_plays_it_when_it_starts() {
    let layer = SoundLayer::new("a.wav");
    assert_eq!(layer.started(None, None, 0.1), Some(0.0));
    assert_eq!(layer.started(None, Some(0.1), 5.0), None);
  }

  #[test]
  fn a_start_outside_a_cycle_is_refused() {
    let timing = Some(Timing::new(1.0, Cycle::Once));
    let late = SoundLayer::new("a.wav").start(1.0).name("hits");
    assert_eq!(late.check(0, timing), Err(SceneError::StartOutsideCycle { layer: "layer \"hits\"".into() }));
    let early = SoundLayer::new("a.wav").start(-0.1);
    assert_eq!(early.check(2, timing), Err(SceneError::StartOutsideCycle { layer: "layer 3".into() }));
    assert_eq!(layer().check(0, timing), Ok(()));
  }

  #[test]
  fn it_can_end_past_the_cycle_but_not_before_it_starts() {
    let timing = Some(Timing::new(1.0, Cycle::Once));
    assert_eq!(layer().end(1.5).check(0, timing), Ok(()));
    assert_eq!(layer().start(0.5).end(1.5).length(), Some(1.0));
    assert_eq!(layer().end(0.4).check(0, timing), Err(SceneError::EndBeforeStart { layer: "layer 1".into() }));
  }

  #[test]
  fn it_can_skip_into_its_sound_but_not_before_it() {
    let timing = Some(Timing::new(1.0, Cycle::Once));
    assert_eq!(layer().skip(0.25).check(0, timing), Ok(()));
    assert_eq!(layer().skip(-0.25).check(0, timing), Err(SceneError::NegativeSkip { layer: "layer 1".into() }));
  }

  #[test]
  fn a_static_scene_can_only_start_it_at_the_top() {
    assert_eq!(layer().check(0, None), Err(SceneError::NoTiming { layer: "layer 1".into() }));
    assert_eq!(SoundLayer::new("a.wav").end(0.5).check(0, None), Ok(()));
  }
}
