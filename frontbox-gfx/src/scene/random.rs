use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::animation::Curve;
use crate::scene::{Animatable, Env};

/// How many times a pick rolls for a value unlike its neighbors' before giving up, as there may not be one
const MAX_ROLLS: u32 = 64;

/// Values picked at random, each held until the next pick or smoothed into it
///
/// A pick is worked out from the moment it's for rather than drawn from a running generator, so that a scene reads the
/// same however a moment is reached and every point sampled in a frame agrees. It still comes out differently on each
/// cycle of the scene and each time the scene is played.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Random {
  /// In order of `at`. The time before the first belongs to the last of the cycle before, or if there isn't one, the first.
  pub picks: Vec<Pick>,
  /// What can be picked
  pub spread: Spread,
  /// Keeps these picks apart from those of another prop picking at the same times
  #[serde(default)]
  pub seed: u32,
  /// Whether a pick has to differ from the one before it. On by default.
  #[serde(default = "on")]
  pub force_unique: bool,
}

impl Random {
  pub(crate) fn resolve<T: Animatable>(&self, env: &Env) -> T {
    self.moving(env, |beat| self.unique(beat, env, false, |beat, attempt| self.roll::<T>(beat, attempt, env)))
  }

  /// [`Self::resolve`] for a number that drops its fraction where it's used, so that picks only count as different if
  /// their whole numbers are. Two whole numbers is the one spread with too few values for [`Self::unique`] to work out
  /// for itself.
  pub(crate) fn resolve_whole(&self, env: &Env) -> f32 {
    let two_values = matches!(self.spread, Spread::Float(low, high) if high.ceil() - low.floor() == 2.0);
    self.moving(env, |beat| self.unique(beat, env, two_values, |beat, attempt| self.roll::<f32>(beat, attempt, env).floor()))
  }

  /// The value now, given the value of each pick
  fn moving<T: Animatable>(&self, env: &Env, value: impl Fn(Beat) -> T) -> T {
    let Some(here) = self.current(env) else {
      return T::default();
    };

    // both picks' smoothing is one blend, so it's eased as one however much of it each gave
    let blend = |from: Beat, to: Beat| {
      let start = self.at(to, env) - self.fades(from, env).1;
      let end = self.at(to, env) + self.fades(to, env).0;
      value(from).interpolate(&value(to), Curve::EaseInOut.sample((env.time - start) / (end - start)))
    };

    let (fade_in, fade_out) = self.fades(here, env);
    match (self.before(here), self.after(here, env)) {
      (Some(before), _) if env.time < self.at(here, env) + fade_in => blend(before, here),
      (_, Some(after)) if env.time > self.at(after, env) - fade_out => blend(here, after),
      _ => value(here),
    }
  }

  /// The value of a pick
  ///
  /// Picks are worked out from their moments, so one can't follow every pick before it to learn what the last came to.
  /// It only has to look next door instead: every other pick keeps its first roll, and those between roll until they
  /// differ from both sides. With only two values those between have no room unless the others agree, so they all take
  /// the first pick's roll, and the two values take turns.
  fn unique<T: PartialEq>(&self, beat: Beat, env: &Env, two_values: bool, roll: impl Fn(Beat, u32) -> T) -> T {
    if !self.force_unique {
      return roll(beat, 0);
    }

    let kept = |beat: Beat| roll(if two_values { Beat { cycle: 0, pick: 0 } } else { beat }, 0);
    if (beat.cycle as u64 * self.picks.len() as u64 + beat.pick as u64) % 2 == 0 {
      return kept(beat);
    }

    let (before, after) = (self.before(beat).map(kept), self.after(beat, env).map(kept));
    (0..MAX_ROLLS)
      .map(|attempt| roll(beat, attempt))
      .find(|value| before.as_ref() != Some(value) && after.as_ref() != Some(value))
      .unwrap_or_else(|| roll(beat, 0))
  }

  fn roll<T: Animatable>(&self, beat: Beat, attempt: u32, env: &Env) -> T {
    T::from_spread(&self.spread, rolls((self.seed, env.take, beat.cycle, beat.pick, attempt))).unwrap_or_default()
  }

  /// The pick that's in effect. `None` if there are none.
  fn current(&self, env: &Env) -> Option<Beat> {
    let last = self.picks.len().checked_sub(1)?;
    Some(match self.picks.partition_point(|pick| pick.at <= env.time) {
      0 if env.cycle == 0 => Beat { cycle: 0, pick: 0 },
      0 => Beat { cycle: env.cycle - 1, pick: last },
      passed => Beat { cycle: env.cycle, pick: passed - 1 },
    })
  }

  fn before(&self, beat: Beat) -> Option<Beat> {
    match beat {
      Beat { cycle: 0, pick: 0 } => None,
      Beat { cycle, pick: 0 } => Some(Beat { cycle: cycle - 1, pick: self.picks.len() - 1 }),
      Beat { cycle, pick } => Some(Beat { cycle, pick: pick - 1 }),
    }
  }

  fn after(&self, beat: Beat, env: &Env) -> Option<Beat> {
    if beat.pick + 1 < self.picks.len() {
      return Some(Beat { pick: beat.pick + 1, ..beat });
    }
    let timing = env.timing?;
    (!timing.is_last_cycle(beat.cycle)).then(|| Beat { cycle: beat.cycle + 1, pick: 0 })
  }

  /// When a pick happens, in seconds from the start of the current cycle
  fn at(&self, beat: Beat, env: &Env) -> f32 {
    let cycles_away = beat.cycle as f32 - env.cycle as f32;
    self.picks[beat.pick].at + cycles_away * env.timing.map_or(0.0, |timing| timing.duration)
  }

  /// How long in seconds a pick takes to arrive and to leave. When they'd take up more than its stretch, they're shortened
  /// to meet. One with nothing after it has nothing to leave for, and its stretch is what's left of the scene.
  fn fades(&self, beat: Beat, env: &Env) -> (f32, f32) {
    let pick = self.picks[beat.pick];
    let after = self.after(beat, env);
    let end = after.map_or(env.timing.map_or(0.0, |timing| timing.duration), |after| self.at(after, env));
    let stretch = (end - self.at(beat, env)).max(0.0);

    let smooth_in = pick.smooth_in.clamp(0.0, 1.0);
    let smooth_out = if after.is_some() { pick.smooth_out.clamp(0.0, 1.0) } else { 0.0 };
    let shorten = (smooth_in + smooth_out).max(1.0);
    (smooth_in * stretch / shorten, smooth_out * stretch / shorten)
  }
}

/// A moment a new value is picked
///
/// It runs until the next pick, that pick's stretch. By default its value holds for all of that, changing at once at each
/// end. Smoothing blends it with the values either side instead, the way a fade does across a cut.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pick {
  /// Seconds into a cycle of the scene
  pub at: f32,
  /// How much of the stretch the value takes to arrive after this, from 0 for at once to 1 for all of it
  #[serde(default)]
  pub smooth_in: f32,
  /// How much of the stretch the value spends leaving for the next pick, from 0 for not until it gets there to 1 for all
  /// of it
  #[serde(default)]
  pub smooth_out: f32,
}

impl Pick {
  pub fn new(at: f32) -> Self {
    Self { at, smooth_in: 0.0, smooth_out: 0.0 }
  }
}

/// What a [`Random`] can pick from. Each pair is the lowest and highest. One of the wrong type gives the type's default.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Spread {
  Float(f32, f32),
  Vec2 {
    x: (f32, f32),
    y: (f32, f32),
  },
  /// Fully saturated colors. Hues are in degrees, so `(330.0, 390.0)` is the reds on both sides of 0. Brightness runs
  /// from 0 for black to 1, and alpha from 0 for transparent to 1.
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

/// One pick, in one cycle of the scene
#[derive(Clone, Copy)]
struct Beat {
  cycle: u32,
  pick: usize,
}

/// Three numbers from 0 to 1 that stand for `moment`
fn rolls(moment: impl Hash) -> [f32; 3] {
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
    assert_ne!(random.resolve::<f32>(&Env { take: 1, ..env(0.0, &params) }), first);
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
    // all of its stretch was asked for twice over, so it arrives and starts to leave halfway through
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

  /// What each pick comes to, as whole numbers, over a scene looping through these picks for `beats` of them
  fn whole_picks(random: &Random, beats: u32) -> Vec<f32> {
    let params = Params::new();
    let timing = Timing::new(1.0, Cycle::Forever);
    let count = random.picks.len() as u32;
    (0..beats)
      .map(|beat| {
        let time = random.picks[(beat % count) as usize].at;
        random.resolve_whole(&Env { cycle: beat / count, timing: Some(timing), ..env(time, &params) })
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
