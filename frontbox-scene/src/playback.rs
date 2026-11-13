use std::collections::BTreeMap;
use std::sync::Arc;

use frontbox_core::prelude::*;
use frontbox_gfx::animation::{Curve, Lerp};
use frontbox_gfx::scene::{Images, Params, Value};

use crate::*;

/// A [`Scene`] being played: the clock, parameters, and everything else that's only true while it plays, none of which is
/// written to a file. The [`SceneSystem`] makes one from a [`SceneContext`] for each scene it's given to play, and
/// systems that draw a kind of layer read them from it.
#[derive(Debug, Clone)]
pub struct Playback {
  scene: Arc<Scene>,
  params: Params,
  /// Params moving toward a value, by name
  easing: BTreeMap<String, Ease>,
  images: Images,
  elapsed: Duration,
  /// How much of the scene plays in a second
  speed: f32,
  playback_seed: u32,
  end_behavior: EndBehavior,
}

impl Playback {
  /// At the start of the scene, with a seed of its own
  pub(crate) fn new<C>(context: &SceneContext<C>) -> Self {
    Self {
      scene: context.scene().clone(),
      params: context.params().clone(),
      easing: BTreeMap::new(),
      images: context.scene_images().clone(),
      elapsed: Duration::ZERO,
      speed: context.speed(),
      playback_seed: rand::random(),
      end_behavior: context.scene_end_behavior(),
    }
  }

  /// Sets a parameter, taking the place of the scene's default, and of any ease toward another value
  pub fn set_param(&mut self, name: impl Into<String>, value: impl Into<Value>) {
    let name = name.into();
    self.easing.remove(&name);
    self.params.insert(name, value.into());
  }

  /// Moves a parameter from where it is now to `to` over `duration`, along `curve`. Where it is now is the value it was
  /// last set or eased to, or else the scene's default. One with neither is set to `to` straight away.
  pub fn ease_param(&mut self, name: impl Into<String>, to: impl Into<Value>, duration: Duration, curve: Curve) {
    let name = name.into();
    let to = to.into();
    let from = self.params.get(&name).or_else(|| self.scene.params.get(&name)).copied();
    match from {
      Some(from) if !duration.is_zero() => {
        self.easing.insert(name, Ease { from, to, elapsed: Duration::ZERO, duration, curve });
      }
      _ => self.set_param(name, to),
    }
  }

  /// Moves time forward by `delta`. Eased parameters keep moving once the scene has completed.
  pub(crate) fn advance(&mut self, delta: Duration) {
    for (name, ease) in &mut self.easing {
      ease.elapsed = (ease.elapsed + delta).min(ease.duration);
      self.params.insert(name.clone(), ease.value());
    }
    self.easing.retain(|_, ease| ease.elapsed < ease.duration);

    if !self.is_complete() {
      self.elapsed += delta.mul_f32(self.speed);
    }
  }

  /// Whether the scene has played through. A scene that's static or repeats forever never completes, and plays until
  /// it's stopped.
  pub fn is_complete(&self) -> bool {
    self.scene.is_complete(self.elapsed.as_secs_f32())
  }

  /// Whether there's anything to show: it hasn't completed, or it holds its last moment
  pub fn is_shown(&self) -> bool {
    self.end_behavior == EndBehavior::Hold || !self.is_complete()
  }

  pub fn scene(&self) -> &Scene {
    &self.scene
  }

  /// Where it is now, for sampling the scene's layers with [`Scene::frame`]
  pub fn moment(&self) -> Moment<'_> {
    Moment { time: self.elapsed.as_secs_f32(), playback_seed: self.playback_seed, params: &self.params, images: &self.images }
  }
}

/// A parameter moving from one value to another
#[derive(Debug, Clone)]
struct Ease {
  from: Value,
  to: Value,
  elapsed: Duration,
  duration: Duration,
  curve: Curve,
}

impl Ease {
  fn value(&self) -> Value {
    let phase = self.elapsed.as_secs_f32() / self.duration.as_secs_f32();
    self.from.interpolate(&self.to, self.curve.sample(phase))
  }
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::scene::Timing;

  use super::*;

  fn timed(duration: f32, cycle: Cycle) -> SceneContext {
    SceneContext::new(Scene::new(vec![]).timing(Timing::new(duration, cycle)))
  }

  fn float(playback: &Playback, name: &str) -> Option<f32> {
    match playback.moment().params.get(name) {
      Some(Value::Float(value)) => Some(*value),
      _ => None,
    }
  }

  #[test]
  fn time_moves_at_the_speed_its_played_at() {
    let mut playback = Playback::new(&timed(1.0, Cycle::Once).over(Duration::from_secs(2)));
    playback.advance(Duration::from_millis(500));
    assert_eq!(playback.moment().time, 0.25);
  }

  #[test]
  fn it_completes_once_every_cycle_has_played() {
    let mut playback = Playback::new(&timed(0.5, Cycle::Times(2)));
    playback.advance(Duration::from_millis(900));
    assert!(!playback.is_complete());
    playback.advance(Duration::from_millis(200));
    assert!(playback.is_complete());
    assert!(!playback.is_shown());
  }

  #[test]
  fn one_that_holds_is_still_shown_once_complete() {
    let mut playback = Playback::new(&timed(0.5, Cycle::Once).end_behavior(EndBehavior::Hold));
    playback.advance(Duration::from_secs(1));
    assert!(playback.is_complete() && playback.is_shown());
  }

  #[test]
  fn each_one_has_a_seed_of_its_own() {
    let context = timed(0.5, Cycle::Once);
    let seeds: Vec<_> = (0..8).map(|_| Playback::new(&context).moment().playback_seed).collect();
    assert!(seeds.iter().any(|seed| *seed != seeds[0]), "{seeds:?}");
  }

  #[test]
  fn an_eased_param_moves_from_where_it_was() {
    let mut playback = Playback::new(&SceneContext::new(Scene::new(vec![])).param("y", 0.0));
    playback.ease_param("y", 10.0, Duration::from_secs(1), Curve::Linear);
    playback.advance(Duration::from_millis(250));
    assert_eq!(float(&playback, "y"), Some(2.5));
    playback.advance(Duration::from_secs(2));
    assert_eq!(float(&playback, "y"), Some(10.0));
  }

  #[test]
  fn an_eased_param_starts_from_the_scenes_default() {
    let mut scene = Scene::new(vec![]);
    scene.params.insert("y".into(), Value::Float(4.0));
    let mut playback = Playback::new(&SceneContext::new(scene));
    playback.ease_param("y", 8.0, Duration::from_secs(1), Curve::Linear);
    playback.advance(Duration::from_millis(500));
    assert_eq!(float(&playback, "y"), Some(6.0));
  }

  #[test]
  fn setting_an_eased_param_ends_the_ease() {
    let mut playback = Playback::new(&SceneContext::new(Scene::new(vec![])).param("y", 0.0));
    playback.ease_param("y", 10.0, Duration::from_secs(1), Curve::Linear);
    playback.set_param("y", 3.0);
    playback.advance(Duration::from_millis(500));
    assert_eq!(float(&playback, "y"), Some(3.0));
  }

  #[test]
  fn a_param_with_nowhere_to_ease_from_is_set() {
    let mut playback = Playback::new(&SceneContext::new(Scene::new(vec![])));
    playback.ease_param("y", 10.0, Duration::from_secs(1), Curve::Linear);
    assert_eq!(float(&playback, "y"), Some(10.0));
  }
}
