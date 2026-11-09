use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::animation::Curve;
use crate::scene::{Animatable, Env};

/// Attempts at a value that differs from its neighbors before giving up (the spread may not have one)
const MAX_RETRIES: u32 = 64;

/// Random values chosen at set times, each held until the next or blended into it
///
/// There's no running random generator. Each value is a hash of the seed, playback seed, cycle, and pick, so any moment
/// can be sampled directly, in any order, with the same result. Values still differ every cycle and every playback.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Random {
  /// Sorted by `at`. Before the first pick, the previous cycle's last pick is in effect (or the first pick, in the first
  /// cycle).
  pub picks: Vec<Pick>,
  pub spread: Spread,
  /// Distinguishes this prop's values from another random prop with the same picks
  #[serde(default)]
  pub seed: u32,
  /// Require each pick to differ from the one before. On by default.
  #[serde(default = "on")]
  pub force_unique: bool,
}

impl Random {
  pub(crate) fn resolve<T: Animatable>(&self, env: &Env) -> T {
    self.blended(env, |instance| self.unique(instance, env, false, |instance, attempt| self.candidate::<T>(instance, attempt, env)))
  }

  /// Like [`Self::resolve`], but `force_unique` compares whole numbers, for values that get truncated. A spread of
  /// exactly two whole numbers needs special handling in [`Self::unique`].
  pub(crate) fn resolve_truncated(&self, env: &Env) -> f32 {
    let two_values = matches!(self.spread, Spread::Float(low, high) if high.ceil() - low.floor() == 2.0);
    self.blended(env, |instance| self.unique(instance, env, two_values, |instance, attempt| self.candidate::<f32>(instance, attempt, env).floor()))
  }

  /// The current value, blending between picks during their smoothing. `value` gives each pick's own value.
  fn blended<T: Animatable>(&self, env: &Env, value: impl Fn(PickInstance) -> T) -> T {
    let Some(here) = self.current(env) else {
      return T::default();
    };

    // one pick's smooth_out and the next one's smooth_in make up a single eased blend
    let blend = |from: PickInstance, to: PickInstance| {
      let start = self.at(to, env) - self.blend_durations(from, env).1;
      let end = self.at(to, env) + self.blend_durations(to, env).0;
      value(from).interpolate(&value(to), Curve::EaseInOut.sample((env.time - start) / (end - start)))
    };

    let (fade_in, fade_out) = self.blend_durations(here, env);
    match (self.before(here), self.after(here, env)) {
      (Some(before), _) if env.time < self.at(here, env) + fade_in => blend(before, here),
      (_, Some(after)) if env.time > self.at(after, env) - fade_out => blend(here, after),
      _ => value(here),
    }
  }

  /// A pick's value, made to differ from its neighbors when `force_unique` is set
  ///
  /// Each pick is computed independently, so a pick can't know what the previous one ended up as. Instead, even picks
  /// keep their first candidate and odd picks try more until one differs from both even neighbors. With only two
  /// possible values, an odd pick only has room if both neighbors match, so every even pick uses the same candidate and
  /// the two values alternate.
  fn unique<T: PartialEq>(&self, instance: PickInstance, env: &Env, two_values: bool, candidate: impl Fn(PickInstance, u32) -> T) -> T {
    if !self.force_unique {
      return candidate(instance, 0);
    }

    let kept = |instance: PickInstance| candidate(if two_values { PickInstance { cycle: 0, pick: 0 } } else { instance }, 0);
    if (instance.cycle as u64 * self.picks.len() as u64 + instance.pick as u64) % 2 == 0 {
      return kept(instance);
    }

    let (before, after) = (self.before(instance).map(kept), self.after(instance, env).map(kept));
    (0..MAX_RETRIES)
      .map(|attempt| candidate(instance, attempt))
      .find(|value| before.as_ref() != Some(value) && after.as_ref() != Some(value))
      .unwrap_or_else(|| candidate(instance, 0))
  }

  /// A random value for a pick. Each `attempt` gives a different value, for retries.
  fn candidate<T: Animatable>(&self, instance: PickInstance, attempt: u32, env: &Env) -> T {
    T::from_spread(&self.spread, rnd((self.seed, env.playback_seed, instance.cycle, instance.pick, attempt))).unwrap_or_default()
  }

  /// The pick that's in effect. `None` if there are none.
  fn current(&self, env: &Env) -> Option<PickInstance> {
    let last = self.picks.len().checked_sub(1)?;
    Some(match self.picks.partition_point(|pick| pick.at <= env.time) {
      0 if env.cycle == 0 => PickInstance { cycle: 0, pick: 0 },
      0 => PickInstance { cycle: env.cycle - 1, pick: last },
      passed => PickInstance { cycle: env.cycle, pick: passed - 1 },
    })
  }

  fn before(&self, instance: PickInstance) -> Option<PickInstance> {
    match instance {
      PickInstance { cycle: 0, pick: 0 } => None,
      PickInstance { cycle, pick: 0 } => Some(PickInstance { cycle: cycle - 1, pick: self.picks.len() - 1 }),
      PickInstance { cycle, pick } => Some(PickInstance { cycle, pick: pick - 1 }),
    }
  }

  fn after(&self, instance: PickInstance, env: &Env) -> Option<PickInstance> {
    if instance.pick + 1 < self.picks.len() {
      return Some(PickInstance { pick: instance.pick + 1, ..instance });
    }
    let timing = env.timing?;
    (!timing.is_last_cycle(instance.cycle)).then(|| PickInstance { cycle: instance.cycle + 1, pick: 0 })
  }

  /// When a pick happens, in seconds from the start of the current cycle (negative for an earlier cycle)
  fn at(&self, instance: PickInstance, env: &Env) -> f32 {
    let cycles_away = instance.cycle as f32 - env.cycle as f32;
    self.picks[instance.pick].at + cycles_away * env.timing.map_or(0.0, |timing| timing.duration)
  }

  /// Seconds a pick spends blending in and blending out. Both are fractions of the time until the next pick, scaled down
  /// to fit if they add up to more than it. The final pick has no blend out, and its time runs to the end of the cycle.
  fn blend_durations(&self, instance: PickInstance, env: &Env) -> (f32, f32) {
    let pick = self.picks[instance.pick];
    let after = self.after(instance, env);
    let end = after.map_or(env.timing.map_or(0.0, |timing| timing.duration), |after| self.at(after, env));
    let span = (end - self.at(instance, env)).max(0.0);

    let smooth_in = pick.smooth_in.clamp(0.0, 1.0);
    let smooth_out = if after.is_some() { pick.smooth_out.clamp(0.0, 1.0) } else { 0.0 };
    let shorten = (smooth_in + smooth_out).max(1.0);
    (smooth_in * span / shorten, smooth_out * span / shorten)
  }
}

/// A time at which a new random value is picked
///
/// The value holds until the next pick, switching instantly at each end. Smoothing blends into and out of it instead.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pick {
  /// Seconds into the scene cycle
  pub at: f32,
  /// Fraction of the time until the next pick spent blending in from the previous value (0 is instant)
  #[serde(default)]
  pub smooth_in: f32,
  /// Fraction of the time until the next pick spent blending out toward the next value (0 is instant)
  #[serde(default)]
  pub smooth_out: f32,
}

impl Pick {
  pub fn new(at: f32) -> Self {
    Self { at, smooth_in: 0.0, smooth_out: 0.0 }
  }
}

/// The range a [`Random`] picks from. Each pair is `(min, max)`. A spread that doesn't match the prop's type gives the
/// type's default.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Spread {
  Float(f32, f32),
  Vec2 {
    x: (f32, f32),
    y: (f32, f32),
  },
  /// Fully saturated HSV colors. Hue is in degrees and can wrap past 360, so `(330.0, 390.0)` covers the reds on both
  /// sides of 0. Brightness (HSV value) and alpha are 0 to 1.
  Color {
    hue: (f32, f32),
    brightness: (f32, f32),
    #[serde(default = "opaque")]
    alpha: (f32, f32),
  },
}

fn on() -> bool {
  true
}

fn opaque() -> (f32, f32) {
  (1.0, 1.0)
}

/// One occurrence of a pick: the pick's index within a particular scene cycle
#[derive(Clone, Copy)]
struct PickInstance {
  cycle: u32,
  pick: usize,
}

/// Three random numbers, each 0 to 1, hashed from `moment`
fn rnd(moment: impl Hash) -> [f32; 3] {
  const MASK: u64 = (1 << 21) - 1;
  let mut hasher = DefaultHasher::new();
  moment.hash(&mut hasher);
  let hash = hasher.finish();
  [0, 21, 42].map(|shift| ((hash >> shift) & MASK) as f32 / MASK as f32)
}

#[cfg(test)]
mod tests {
  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::animation::Cycle;
  use crate::scene::value::tests::env;
  use crate::scene::{Color, Params, Prop, Timing};

  fn random(picks: Vec<f32>) -> Random {
    Random {
      picks: picks.into_iter().map(Pick::new).collect(),
      spread: Spread::Float(0.0, 1000.0),
      seed: 1,
      force_unique: true,
    }
  }

  /// The value of a [`random`] with these picks, at a moment of a scene 4 seconds long
  fn smoothed(picks: &[Pick], cycle: Cycle, scene_time: f32) -> f32 {
    let params = Params::new();
    let timing = Timing::new(4.0, cycle);
    let env = Env {
      cycle: timing.cycle_index(scene_time),
      timing: Some(timing),
      ..env(timing.local_time(scene_time), &params)
    };
    Random { picks: picks.to_vec(), ..random(vec![]) }.resolve(&env)
  }

  fn pick(at: f32, smooth_in: f32, smooth_out: f32) -> Pick {
    Pick { at, smooth_in, smooth_out }
  }

  #[test]
  fn holds_a_value_until_the_next_pick() {
    let params = Params::new();
    let random = random(vec![0.0, 1.0]);
    let first: f32 = random.resolve(&env(0.0, &params));
    assert_eq!(random.resolve::<f32>(&env(0.5, &params)), first);
    assert_eq!(random.resolve::<f32>(&env(0.99, &params)), first);
    assert_ne!(random.resolve::<f32>(&env(1.0, &params)), first);
    assert_eq!(random.resolve::<f32>(&env(1.0, &params)), random.resolve::<f32>(&env(50.0, &params)));
  }

  #[test]
  fn the_first_pick_covers_the_time_before_it() {
    let params = Params::new();
    let random = random(vec![1.0]);
    assert_eq!(random.resolve::<f32>(&env(0.0, &params)), random.resolve::<f32>(&env(1.0, &params)));
  }

  #[test]
  fn each_cycle_and_each_take_picks_again() {
    let params = Params::new();
    let random = random(vec![0.0]);
    let first: f32 = random.resolve(&env(0.0, &params));
    assert_ne!(random.resolve::<f32>(&Env { cycle: 1, ..env(0.0, &params) }), first);
    assert_ne!(random.resolve::<f32>(&Env { playback_seed: 1, ..env(0.0, &params) }), first);
  }

  #[test]
  fn seeds_keep_props_apart() {
    let params = Params::new();
    let other = Random { seed: 2, ..random(vec![0.0]) };
    assert_ne!(random(vec![0.0]).resolve::<f32>(&env(0.0, &params)), other.resolve::<f32>(&env(0.0, &params)));
  }

  #[test]
  fn stays_in_its_spread() {
    let params = Params::new();
    let spread = Spread::Vec2 { x: (2.0, 3.0), y: (-5.0, -4.0) };
    for seed in 0..500 {
      let value: Vec2 = Random { picks: vec![Pick::new(0.0)], spread, seed, force_unique: true }.resolve(&env(0.0, &params));
      assert!((2.0..=3.0).contains(&value.x), "{value}");
      assert!((-5.0..=-4.0).contains(&value.y), "{value}");
    }
  }

  #[test]
  fn an_empty_spread_is_that_value() {
    let params = Params::new();
    let random = Random { picks: vec![Pick::new(0.0)], spread: Spread::Float(4.0, 4.0), seed: 1, force_unique: true };
    assert_eq!(random.resolve::<f32>(&env(0.0, &params)), 4.0);
  }

  #[test]
  fn colors_are_picked_by_hue_and_brightness() {
    let params = Params::new();
    let color = |hue, brightness| -> Rgba<u8> {
      let spread = Spread::Color { hue, brightness, alpha: (1.0, 1.0) };
      Random { picks: vec![Pick::new(0.0)], spread, seed: 1, force_unique: true }.resolve::<Color>(&env(0.0, &params)).0
    };
    assert_eq!(color((120.0, 120.0), (1.0, 1.0)), Rgba([0, 255, 0, 255]));
    assert_eq!(color((240.0, 240.0), (0.5, 0.5)), Rgba([0, 0, 128, 255]));
    assert_eq!(color((0.0, 360.0), (0.0, 0.0)), Rgba([0, 0, 0, 255]));
  }

  #[test]
  fn hues_can_span_the_reds_on_both_sides_of_zero() {
    let params = Params::new();
    for seed in 0..500 {
      let spread = Spread::Color { hue: (330.0, 390.0), brightness: (1.0, 1.0), alpha: (1.0, 1.0) };
      let picks = vec![Pick::new(0.0)];
      let Rgba([red, green, blue, _]) = Random { picks, spread, seed, force_unique: true }.resolve::<Color>(&env(0.0, &params)).0;
      assert_eq!(red, 255);
      assert!(green <= 128 && blue <= 128, "{green} {blue}");
    }
  }

  #[test]
  fn alpha_stays_in_its_spread_and_comes_apart_from_the_color() {
    let params = Params::new();
    let spread = Spread::Color { hue: (120.0, 120.0), brightness: (1.0, 1.0), alpha: (0.25, 0.75) };
    let alphas: Vec<u8> = (0..500)
      .map(|seed| {
        let Rgba([red, green, blue, alpha]) =
          Random { picks: vec![Pick::new(0.0)], spread, seed, force_unique: true }.resolve::<Color>(&env(0.0, &params)).0;
        assert_eq!((red, green, blue), (0, 255, 0));
        alpha
      })
      .collect();
    assert!(alphas.iter().all(|alpha| (64..=191).contains(alpha)), "{alphas:?}");
    assert!(alphas.iter().any(|alpha| *alpha != alphas[0]));
  }

  #[test]
  fn with_no_smoothing_a_value_changes_at_once() {
    let picks = [Pick::new(0.0), Pick::new(2.0)];
    assert_eq!(smoothed(&picks, Cycle::Once, 1.99), smoothed(&picks, Cycle::Once, 0.0));
    assert_eq!(smoothed(&picks, Cycle::Once, 2.0), smoothed(&picks, Cycle::Once, 3.0));
  }

  #[test]
  fn smoothing_in_arrives_after_the_pick() {
    let picks = [Pick::new(0.0), pick(2.0, 0.5, 0.0)];
    let (from, to) = (smoothed(&picks, Cycle::Once, 1.99), smoothed(&picks, Cycle::Once, 3.5));
    assert_eq!(smoothed(&picks, Cycle::Once, 2.0), from);
    assert_eq!(smoothed(&picks, Cycle::Once, 3.0), to);
    let halfway = smoothed(&picks, Cycle::Once, 2.5);
    assert!((halfway - (from + to) / 2.0).abs() < 1e-3, "{from} {halfway} {to}");
  }

  #[test]
  fn smoothing_out_leaves_before_the_next_pick() {
    let picks = [pick(0.0, 0.0, 0.5), Pick::new(2.0)];
    let (from, to) = (smoothed(&picks, Cycle::Once, 0.5), smoothed(&picks, Cycle::Once, 3.0));
    assert_eq!(smoothed(&picks, Cycle::Once, 1.0), from);
    assert_eq!(smoothed(&picks, Cycle::Once, 2.0), to);
    let halfway = smoothed(&picks, Cycle::Once, 1.5);
    assert!((halfway - (from + to) / 2.0).abs() < 1e-3, "{from} {halfway} {to}");
  }

  #[test]
  fn smoothing_on_both_sides_of_a_pick_is_one_blend_with_no_jump() {
    let picks = [pick(0.0, 0.0, 0.5), pick(2.0, 0.5, 0.0)];
    let (from, to) = (smoothed(&picks, Cycle::Once, 0.5), smoothed(&picks, Cycle::Once, 3.5));
    let halfway = smoothed(&picks, Cycle::Once, 2.0);
    assert!((halfway - (from + to) / 2.0).abs() < 1e-3, "{from} {halfway} {to}");
    let moments: Vec<f32> = (100..=300).map(|n| smoothed(&picks, Cycle::Once, n as f32 / 100.0)).collect();
    let most = (to - from).abs() / 40.0;
    assert!(moments.windows(2).all(|pair| (pair[1] - pair[0]).abs() < most), "{moments:?}");
  }

  #[test]
  fn smoothing_too_much_shortens_it_so_a_pick_still_arrives() {
    let picks = [pick(0.0, 0.0, 1.0), pick(1.0, 1.0, 1.0), pick(2.0, 1.0, 0.0)];
    let unsmoothed = [Pick::new(0.0), Pick::new(1.0), Pick::new(2.0)];
    // all of its span was asked for twice over, so it arrives and starts to leave halfway through
    assert_eq!(smoothed(&picks, Cycle::Once, 1.5), smoothed(&unsmoothed, Cycle::Once, 1.5));
  }

  #[test]
  fn a_loop_smooths_across_into_the_next_cycle() {
    let picks = [pick(0.0, 0.5, 0.0)];
    let (from, to) = (smoothed(&picks, Cycle::Forever, 3.9), smoothed(&picks, Cycle::Forever, 7.9));
    assert_eq!(smoothed(&picks, Cycle::Forever, 4.0), from);
    let halfway = smoothed(&picks, Cycle::Forever, 5.0);
    assert!((halfway - (from + to) / 2.0).abs() < 1e-3, "{from} {halfway} {to}");
    assert_eq!(smoothed(&picks, Cycle::Forever, 6.0), to);
  }

  #[test]
  fn a_loop_holds_its_last_pick_until_the_first_of_the_next_cycle() {
    let picks = [Pick::new(1.0)];
    assert_eq!(smoothed(&picks, Cycle::Forever, 4.5), smoothed(&picks, Cycle::Forever, 4.0));
    assert_eq!(smoothed(&picks, Cycle::Forever, 4.5), smoothed(&picks, Cycle::Forever, 3.0));
    assert_ne!(smoothed(&picks, Cycle::Forever, 5.0), smoothed(&picks, Cycle::Forever, 4.5));
  }

  #[test]
  fn the_first_and_last_moments_have_nothing_to_smooth_with() {
    let picks = [pick(0.0, 0.5, 0.5)];
    assert_eq!(smoothed(&picks, Cycle::Once, 0.0), smoothed(&picks, Cycle::Once, 4.0));
    let twice = Cycle::Times(2);
    assert_eq!(smoothed(&picks, twice, 0.0), smoothed(&picks, twice, 1.0));
    assert_eq!(smoothed(&picks, twice, 6.0), smoothed(&picks, twice, 8.0));
    assert_ne!(smoothed(&picks, twice, 3.0), smoothed(&picks, twice, 3.9));
  }

  /// What each pick comes to, as whole numbers, over a scene looping through these picks for `instances` of them
  fn whole_picks(random: &Random, instances: u32) -> Vec<f32> {
    let params = Params::new();
    let timing = Timing::new(1.0, Cycle::Forever);
    let count = random.picks.len() as u32;
    (0..instances)
      .map(|instance| {
        let time = random.picks[(instance % count) as usize].at;
        random.resolve_truncated(&Env { cycle: instance / count, timing: Some(timing), ..env(time, &params) })
      })
      .collect()
  }

  fn numbers(picks: usize, count: u32, seed: u32) -> Random {
    let picks = (0..picks).map(|n| n as f32 / picks as f32).collect();
    Random { spread: Spread::Float(0.0, count as f32), seed, ..random(picks) }
  }

  #[test]
  fn a_whole_number_never_comes_up_twice_in_a_row() {
    for (picks, count, seed) in (1..=3).flat_map(|p| (2..=6).flat_map(move |c| (0..20).map(move |s| (p, c, s)))) {
      let picks = whole_picks(&numbers(picks, count, seed), 200);
      assert!(picks.windows(2).all(|pair| pair[0] != pair[1]), "{picks:?}");
      assert!(picks.iter().all(|pick| (0.0..count as f32).contains(pick)), "{picks:?}");
    }
  }

  #[test]
  fn forcing_uniqueness_still_reaches_every_value_about_as_often() {
    let picks = whole_picks(&numbers(1, 4, 1), 8000);
    for value in 0..4 {
      let seen = picks.iter().filter(|pick| **pick == value as f32).count();
      assert!((1600..2400).contains(&seen), "{value} came up {seen} times");
    }
  }

  #[test]
  fn two_values_take_turns() {
    let picks = whole_picks(&numbers(1, 2, 1), 50);
    assert!(picks.windows(3).all(|three| three[0] == three[2] && three[0] != three[1]), "{picks:?}");
  }

  #[test]
  fn without_force_unique_a_value_can_repeat() {
    let random = Random { force_unique: false, ..numbers(1, 2, 1) };
    let picks = whole_picks(&random, 50);
    assert!(picks.windows(2).any(|pair| pair[0] == pair[1]), "{picks:?}");
  }

  #[test]
  fn colors_differ_from_the_one_before_too() {
    let params = Params::new();
    let timing = Timing::new(1.0, Cycle::Forever);
    // only a few colors are this dark
    let spread = Spread::Color { hue: (120.0, 120.0), brightness: (0.0, 0.016), alpha: (1.0, 1.0) };
    let random = Random { spread, ..random(vec![0.0]) };
    let colors: Vec<Color> =
      (0..200).map(|cycle| random.resolve(&Env { cycle, timing: Some(timing), ..env(0.0, &params) })).collect();
    assert!(colors.windows(2).all(|pair| pair[0] != pair[1]));
  }

  #[test]
  fn a_spread_of_one_value_is_that_value_at_every_pick() {
    assert_eq!(whole_picks(&numbers(1, 1, 1), 10), vec![0.0; 10]);
  }

  #[test]
  fn nothing_to_pick_from_is_the_default() {
    let params = Params::new();
    assert_eq!(random(vec![]).resolve::<f32>(&env(0.0, &params)), 0.0);
    assert_eq!(random(vec![0.0]).resolve::<Color>(&env(0.0, &params)), Color::default());
  }

  #[test]
  fn a_prop_reads_its_picks_and_is_animated() {
    let params = Params::new();
    let prop: Prop<f32> = Prop::Random(random(vec![0.0]));
    assert!(prop.is_animated());
    assert_eq!(prop.resolve(&env(0.0, &params)), random(vec![0.0]).resolve::<f32>(&env(0.0, &params)));
  }
}
