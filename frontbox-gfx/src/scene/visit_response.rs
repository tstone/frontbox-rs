use serde::{Deserialize, Serialize};

use crate::animation::Curve;
use crate::scene::{Animatable, Env};

/// A value that moves from `from` to `to` each time a traversal visits the node being drawn, then back
///
/// Timing is in the traversal's steps, so the response keeps its shape at any rate: `smooth_out: Some(3.0)` leaves a
/// trail three steps long. For a traversal that goes by distance, a step is one scene unit of travel. Outside a layer
/// with a graph, or for a traversal the layer doesn't have, it stays at `from`.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisitResponse<T> {
  /// The traversal's name, in the layer's [`GraphMask`](crate::scene::GraphMask)
  pub traversal: String,
  /// With no visit in effect
  pub from: T,
  /// At the full response to a visit
  pub to: T,
  /// Steps from a visit to reaching `to`
  #[serde(default)]
  pub smooth_in: f32,
  /// Steps held at `to` after reaching it
  #[serde(default)]
  pub hold: f32,
  /// Steps back to `from` after the hold. `None` stays at `to`.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub smooth_out: Option<f32>,
  #[serde(default)]
  pub curve_in: Curve,
  /// Played backwards on the way out, so an ease-in curve leaves `to` quickly and settles slowly into `from`
  #[serde(default)]
  pub curve_out: Curve,
  #[serde(default)]
  pub overlap: Overlap,
}

/// How a node combines responses to visits close enough together to overlap
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum Overlap {
  /// The strongest response wins
  #[default]
  Max,
  /// Responses add up, going past `to` for overlapping visits: with `smooth_out: None`, each visit moves another full
  /// step from `from` toward and past `to`
  Add,
}

impl<T: Animatable> VisitResponse<T> {
  pub fn new(traversal: impl Into<String>, from: T, to: T) -> Self {
    Self {
      traversal: traversal.into(),
      from,
      to,
      smooth_in: 0.0,
      hold: 0.0,
      smooth_out: None,
      curve_in: Curve::Linear,
      curve_out: Curve::Linear,
      overlap: Overlap::Max,
    }
  }

  pub(crate) fn resolve(&self, env: &Env) -> T {
    let Some((visits, rate)) = env.visits.and_then(|visits| visits.of(&self.traversal)) else {
      return self.from.clone();
    };
    self.from.interpolate(&self.to, self.level(visits, rate, env.time))
  }

  /// The response at `time` to visits at the sorted times `visits`, from 0 to 1, or more where `Add` stacks them
  fn level(&self, visits: &[f32], rate: f32, time: f32) -> f32 {
    let length = self.smooth_out.map_or(f32::INFINITY, |out| self.smooth_in + self.hold + out);
    let passed = &visits[..visits.partition_point(|&at| at <= time)];
    let levels = passed.iter().rev().map(|&at| (time - at) * rate).take_while(|&steps| steps < length).map(|steps| self.single(steps));
    match self.overlap {
      Overlap::Max => levels.fold(0.0, f32::max),
      Overlap::Add => levels.sum(),
    }
  }

  /// The response to one visit, `steps` after it
  fn single(&self, steps: f32) -> f32 {
    if steps < self.smooth_in {
      return self.curve_in.sample(steps / self.smooth_in);
    }
    let after_hold = steps - self.smooth_in - self.hold;
    match self.smooth_out {
      Some(out) if after_hold >= 0.0 => self.curve_out.sample(1.0 - (after_hold / out).min(1.0)),
      _ => 1.0,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Params, TraversalVisits, Visits};

  /// Visits at these seconds to node 0 from traversal "t", stepping twice a second
  fn visited(at: &[f32]) -> Visits {
    Visits { layers: vec![vec![TraversalVisits { name: "t".into(), rate: 2.0, by_node: vec![at.to_vec()], ends: None }]] }
  }

  fn at(response: &VisitResponse<f32>, visits: &Visits, time: f32) -> f32 {
    let params = Params::new();
    response.resolve(&Env { visits: visits.at(0, 0), ..env(time, &params) })
  }

  fn response() -> VisitResponse<f32> {
    VisitResponse { smooth_in: 1.0, hold: 1.0, smooth_out: Some(2.0), ..VisitResponse::new("t", 0.0, 10.0) }
  }

  #[test]
  fn rises_holds_and_falls_in_steps() {
    let visits = visited(&[1.0]);
    let response = response();
    // twice a second: in from 1.0 to 1.5, held to 2.0, out by 3.0
    let samples: Vec<f32> = [0.5, 1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 2.75, 3.0].iter().map(|&time| at(&response, &visits, time)).collect();
    assert_eq!(samples, [0.0, 0.0, 5.0, 10.0, 10.0, 10.0, 7.5, 5.0, 2.5, 0.0]);
  }

  #[test]
  fn with_no_smooth_out_it_stays() {
    let response = VisitResponse { smooth_out: None, ..response() };
    assert_eq!(at(&response, &visited(&[1.0]), 50.0), 10.0);
  }

  #[test]
  fn the_out_curve_plays_backwards() {
    let response = VisitResponse { smooth_in: 0.0, hold: 0.0, smooth_out: Some(2.0), curve_out: Curve::EaseIn, ..response() };
    // half way out: EaseIn at 0.5 is 0.25
    assert_eq!(at(&response, &visited(&[0.0]), 0.5), 2.5);
  }

  #[test]
  fn overlapping_visits_take_the_strongest_or_add_up() {
    let visits = visited(&[0.0, 0.25]);
    let max = response();
    let add = VisitResponse { overlap: Overlap::Add, ..response() };
    // 0.5s in: the first visit is a step in, at 10; the second half a step in, at 5
    assert_eq!(at(&max, &visits, 0.5), 10.0);
    assert_eq!(at(&add, &visits, 0.5), 15.0);
  }

  #[test]
  fn adding_with_no_smooth_out_counts_visits() {
    let response = VisitResponse { overlap: Overlap::Add, ..VisitResponse::new("t", 1.0, 1.1) };
    assert!((at(&response, &visited(&[0.0, 1.0, 2.0]), 5.0) - 1.3).abs() < 1e-5);
  }

  #[test]
  fn without_visits_or_with_another_traversal_it_stays_at_from() {
    let params = Params::new();
    assert_eq!(response().resolve(&env(1.0, &params)), 0.0);
    let other = VisitResponse { traversal: "other".into(), ..response() };
    assert_eq!(at(&other, &visited(&[0.0]), 0.5), 0.0);
  }
}
