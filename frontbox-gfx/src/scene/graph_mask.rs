use std::collections::BinaryHeap;
use std::cmp::Ordering;

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::{Env, Graph, Prop, TraversalVisits, rnd};

/// Most steps a random walk takes in one frame, so a huge rate can't stall playback
const MAX_WALK_STEPS: usize = 100_000;

/// Limits a layer to the nodes of a graph, with traversals moving over it
///
/// The graph is built by the device from where its points are, with [`Graph::build`]. Samples that aren't a node of it
/// are left out of the layer. [`Prop::Visit`] responds to a traversal's visits, so an opacity set to one shows the layer
/// where the traversal has been.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphMask {
  #[serde(default)]
  pub traversals: Vec<Traversal>,
}

impl GraphMask {
  pub fn new(traversals: Vec<Traversal>) -> Self {
    Self { traversals }
  }

  pub(crate) fn is_animated(&self) -> bool {
    !self.traversals.is_empty()
  }
}

/// A walk over a graph, visiting nodes at a set speed, starting from one node
///
/// Visits are worked out fresh each cycle, so random choices differ by cycle and playback, as with [`Random`](crate::scene::Random).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traversal {
  /// How a [`VisitResponse`](crate::scene::VisitResponse) refers to it. Unique within the layer.
  pub name: String,
  pub from: NodeRef,
  pub algorithm: TraversalAlgorithm,
  pub speed: Speed,
  /// Seconds into the cycle when it leaves `from`
  #[serde(default)]
  pub start: f32,
}

impl Traversal {
  pub fn new(name: impl Into<String>, from: NodeRef, algorithm: TraversalAlgorithm, speed: impl Into<Speed>) -> Self {
    Self { name: name.into(), from, algorithm, speed: speed.into(), start: 0.0 }
  }

  pub fn start(mut self, start: f32) -> Self {
    self.start = start;
    self
  }

  /// Every visit it makes this cycle, including ones still to come
  pub(crate) fn visits(&self, graph: &Graph, env: &Env) -> TraversalVisits {
    let nothing = |rate| TraversalVisits { name: self.name.clone(), rate, by_node: vec![Vec::new(); graph.nodes().len()], ends: None };
    let rate = match &self.speed {
      Speed::Rate(rate) => Some(rate.resolve(env)),
      Speed::Duration(_) => None,
    };
    let Some(from) = self.from.resolve(graph, env).filter(|_| rate.is_none_or(|rate| rate > 0.0)) else {
      return nothing(rate.unwrap_or(0.0));
    };

    // each visit, in steps from leaving `from`
    let mut steps: Vec<(usize, f32)> = Vec::new();
    let seed = (&self.name, env.playback_seed, env.cycle);
    // wander is read at the start of the cycle, so the plan is the same whichever frame makes it
    let cycle_start = Env { time: 0.0, ..*env };
    match &self.algorithm {
      TraversalAlgorithm::BreadthFirst { wander } | TraversalAlgorithm::Dijkstra { wander } => {
        let by_steps = matches!(self.algorithm, TraversalAlgorithm::BreadthFirst { .. });
        let edge = |length| if by_steps { 1.0 } else { length };
        let wander = wander.resolve(&cycle_start);
        let (mut arrivals, _) = shortest_paths(graph, from, |_, _, length| edge(length));
        if wander > 0.0 {
          let last = |arrivals: &[f32]| arrivals.iter().copied().filter(|at| at.is_finite()).fold(0.0, f32::max);
          let factor = edge_factor(wander, seed);
          let (wandering, _) = shortest_paths(graph, from, |a, b, length| edge(length) * factor(a, b));
          // quick edges make it finish early, so it's stretched to end when the even fill would
          let stretch = last(&arrivals) / last(&wandering).max(f32::EPSILON);
          arrivals = wandering.into_iter().map(|at| at * stretch).collect();
        }
        steps.extend(arrivals.into_iter().enumerate().filter(|(_, at)| at.is_finite()));
      }
      TraversalAlgorithm::DepthFirst => {
        steps.extend(depth_first(graph, from, seed).into_iter().enumerate().map(|(step, node)| (node, step as f32)));
      }
      TraversalAlgorithm::ShortestPath { to, wander } => {
        let wander = wander.resolve(&cycle_start);
        let path = to.resolve(graph, env).map_or(Vec::new(), |to| wandering_path(graph, from, to, wander, seed));
        steps.extend(path_steps(graph, &path));
      }
      TraversalAlgorithm::FanOut { to, angle, count, wander } => {
        let wander = wander.resolve(&cycle_start);
        let ends = to.resolve(graph, env).map_or(Vec::new(), |to| {
          fan_ends(graph, from, to, angle.resolve(&cycle_start), count.resolve(&cycle_start), seed)
        });
        for (index, end) in ends.into_iter().enumerate() {
          steps.extend(path_steps(graph, &wandering_path(graph, from, end, wander, (seed, index))));
        }
        // paths that start out the same visit those nodes at the same moment, which is one visit
        steps.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        steps.dedup();
      }
      TraversalAlgorithm::AStar { to } => {
        let to = to.resolve(graph, env);
        steps.extend(a_star_order(graph, from, to).into_iter().enumerate().map(|(step, node)| (node, step as f32)));
      }
      TraversalAlgorithm::RandomWalk => {
        let walk = match rate {
          Some(rate) => {
            let cycle_end = env.timing.map_or(env.time, |timing| timing.duration);
            ((cycle_end - self.start) * rate).floor().max(0.0) as usize
          }
          None => graph.nodes().len(),
        };
        steps.extend(random_walk(graph, from, walk.min(MAX_WALK_STEPS), seed).into_iter().enumerate().map(|(step, node)| (node, step as f32)));
      }
    }

    let rate = match &self.speed {
      Speed::Rate(_) => rate.unwrap_or_default(),
      Speed::Duration(duration) => {
        let duration = duration.unwrap_or_else(|| env.timing.map_or(0.0, |timing| timing.duration) - self.start);
        let longest = steps.iter().map(|&(_, steps)| steps).fold(0.0, f32::max);
        // the last node gets an average step of its own before time's up. A repeating cycle wraps at its end, so a
        // visit right at the end would never show.
        let mut distinct: Vec<f32> = steps.iter().map(|&(_, steps)| steps).collect();
        distinct.sort_by(f32::total_cmp);
        distinct.dedup();
        let last_step = longest / (distinct.len().max(2) - 1) as f32;
        // a traversal that never leaves `from` has no length to stretch, so any rate does
        if duration <= 0.0 { 0.0 } else if longest > 0.0 { (longest + last_step) / duration } else { 1.0 }
      }
    };
    if rate <= 0.0 {
      return nothing(rate);
    }

    let mut by_node = vec![Vec::new(); graph.nodes().len()];
    for (node, steps) in steps {
      by_node[node].push(self.start + steps / rate);
    }
    by_node.iter_mut().for_each(|times| times.sort_by(f32::total_cmp));
    // the node visited last, the lowest index on a tie
    let last = by_node.iter().enumerate().filter_map(|(node, times)| Some((node, *times.last()?)));
    let last = last.fold(None, |best: Option<(usize, f32)>, (node, at)| match best {
      Some((_, best_at)) if best_at >= at => best,
      _ => Some((node, at)),
    });
    let ends = last.map(|(end, _)| (graph.nodes()[from].pos, graph.nodes()[end].pos));
    TraversalVisits { name: self.name.clone(), rate, by_node, ends }
  }
}

/// How fast a traversal goes
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Speed {
  /// Steps per second, or for algorithms that go by distance, scene units per second. Nothing is visited unless it's
  /// above 0.
  Rate(Prop<f32>),
  /// Seconds from leaving `from` to the end of the last visit, however far that is, with the last node shown for an
  /// average step. `None` takes the rest of the cycle. A random walk takes as many steps as the graph has nodes.
  Duration(Option<f32>),
}

impl From<f32> for Speed {
  fn from(rate: f32) -> Self {
    Self::Rate(Prop::Fixed(rate))
  }
}

impl From<Prop<f32>> for Speed {
  fn from(rate: Prop<f32>) -> Self {
    Self::Rate(rate)
  }
}

/// Which nodes a traversal visits, in what order, and how far apart in time
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraversalAlgorithm {
  /// Every reachable node, by the number of edges from the start: a step is one edge, so nodes the same number of edges
  /// away are visited together
  BreadthFirst {
    /// How unevenly it spreads. Above 0, each edge takes a random number of steps between e^-wander and e^wander, so
    /// the fill runs ahead along quick edges, stretched to end when it would without wander. Read once a cycle, at its
    /// start.
    #[serde(default = "no_wander")]
    wander: Prop<f32>,
  },
  /// Every reachable node, one per step, following one branch to its end before backing up to the next. Branches are
  /// taken in a random order.
  DepthFirst,
  /// Every reachable node, by path distance from the start. Goes by distance.
  Dijkstra {
    /// How unevenly it spreads, as for [`TraversalAlgorithm::BreadthFirst`], with each edge's random factor applied to
    /// its length
    #[serde(default = "no_wander")]
    wander: Prop<f32>,
  },
  /// Only the nodes on the shortest path to `to`. Goes by distance. Nothing is visited if `to` can't be reached.
  ShortestPath {
    to: NodeRef,
    /// How freely the path wanders off the shortest one. 0 always takes the shortest. Above it, the path is a random
    /// walk toward `to` with its loops cut out, steered less the higher it is. Around 5 it's barely steered at all. Read
    /// once a cycle, at its start, so a track or random value gives each cycle its own amount.
    #[serde(default = "no_wander")]
    wander: Prop<f32>,
  },
  /// Paths out from the start to `to` and `count` more nodes, all at once. The others are picked at random along an arc
  /// around the start through `to`, `angle` degrees wide and centered on it, each nearest a point in its own equal part
  /// of the arc so they spread across it. Each path is a [`TraversalAlgorithm::ShortestPath`] with `wander`. Goes by
  /// distance. Nothing is visited if `to` can't be found.
  FanOut {
    to: NodeRef,
    /// Degrees the arc spans. Read once a cycle, at its start, as `count` and `wander` are.
    angle: Prop<f32>,
    /// How many more nodes to go to besides `to`, rounded
    count: Prop<f32>,
    #[serde(default = "no_wander")]
    wander: Prop<f32>,
  },
  /// The nodes an A* search for `to` explores, one per step in the order it settles them, ending at `to`. Without a
  /// reachable `to`, it explores everything reachable.
  AStar { to: NodeRef },
  /// One random neighbor per step, not going straight back unless it's at a dead end. Nodes can be visited many times.
  /// The whole cycle's walk is chosen at its start, so it has a known end.
  RandomWalk,
}

/// A wander when a file leaves it out
fn no_wander() -> Prop<f32> {
  Prop::Fixed(0.0)
}

/// A node of the graph, by name or by position
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeRef {
  Name(String),
  /// The node closest to a position in scene units. Resolved every frame, so it's meant to be fixed or a param.
  Nearest(#[cfg_attr(feature = "ts", ts(as = "Prop<[f32; 2]>"))] Prop<Vec2>),
}

impl NodeRef {
  pub fn name(name: impl Into<String>) -> Self {
    Self::Name(name.into())
  }

  fn resolve(&self, graph: &Graph, env: &Env) -> Option<usize> {
    match self {
      Self::Name(name) => graph.find(name),
      Self::Nearest(pos) => graph.nearest(pos.resolve(env)),
    }
  }
}

/// A random factor for each edge, the same both ways, log-uniform between e^-wander and e^wander
fn edge_factor(wander: f32, seed: impl std::hash::Hash + Copy) -> impl Fn(usize, usize) -> f32 {
  move |a, b| (wander * (2.0 * rnd((seed, a.min(b), a.max(b)))[0] - 1.0)).exp()
}

/// Preorder, taking each node's neighbors in a random order
fn depth_first(graph: &Graph, from: usize, seed: impl std::hash::Hash + Copy) -> Vec<usize> {
  let mut seen = vec![false; graph.nodes().len()];
  let mut order = Vec::new();
  let mut stack = vec![from];
  while let Some(node) = stack.pop() {
    if std::mem::replace(&mut seen[node], true) {
      continue;
    }
    order.push(node);
    let mut next: Vec<usize> = graph.neighbors(node).iter().map(|(next, _)| *next).filter(|&next| !seen[next]).collect();
    // pushed so the lowest random number comes off the stack first
    next.sort_by(|&a, &b| rnd((seed, node, b))[0].total_cmp(&rnd((seed, node, a))[0]));
    stack.extend(next);
  }
  order
}

/// Dijkstra from `from`, with each edge weighted by `weight(a, b, length)`: each node's distance (infinite where it
/// can't be reached) and the node before it on its shortest path
fn shortest_paths(graph: &Graph, from: usize, weight: impl Fn(usize, usize, f32) -> f32) -> (Vec<f32>, Vec<Option<usize>>) {
  let mut distances = vec![f32::INFINITY; graph.nodes().len()];
  let mut previous = vec![None; graph.nodes().len()];
  let mut queue = BinaryHeap::from([Queued { cost: 0.0, node: from }]);
  distances[from] = 0.0;
  while let Some(Queued { cost, node }) = queue.pop() {
    if cost > distances[node] {
      continue;
    }
    for &(next, length) in graph.neighbors(node) {
      let through = cost + weight(node, next, length);
      if through < distances[next] {
        distances[next] = through;
        previous[next] = Some(node);
        queue.push(Queued { cost: through, node: next });
      }
    }
  }
  (distances, previous)
}

/// Each node on `path` with its distance along it
fn path_steps(graph: &Graph, path: &[usize]) -> Vec<(usize, f32)> {
  let mut distance = 0.0;
  let mut steps: Vec<(usize, f32)> = path.first().map(|&node| (node, 0.0)).into_iter().collect();
  for pair in path.windows(2) {
    distance += graph.nodes()[pair[0]].pos.distance(graph.nodes()[pair[1]].pos);
    steps.push((pair[1], distance));
  }
  steps
}

/// Where a [`TraversalAlgorithm::FanOut`] goes: `to`, then a node in each of `count` parts of the arc, without repeats
fn fan_ends(graph: &Graph, from: usize, to: usize, angle: f32, count: f32, seed: impl std::hash::Hash + Copy) -> Vec<usize> {
  let center = graph.nodes()[from].pos;
  let reach = graph.nodes()[to].pos - center;
  let count = count.round().max(0.0) as usize;
  let mut ends = vec![to];
  for part in 0..count {
    let along = (part as f32 + rnd((seed, "fan", part))[0]) / count as f32 - 0.5;
    let end = graph.nearest(center + Vec2::from_angle(along * angle.to_radians()).rotate(reach));
    if let Some(end) = end.filter(|end| !ends.contains(end)) {
      ends.push(end);
    }
  }
  ends
}

/// The nodes from `from` to `to`, following `previous` back. Empty if `to` can't be reached.
fn path_to(previous: &[Option<usize>], from: usize, to: usize) -> Vec<usize> {
  let mut path = vec![to];
  while let Some(&last) = path.last().filter(|&&last| last != from) {
    match previous[last] {
      Some(node) => path.push(node),
      None => return Vec::new(),
    }
  }
  path.reverse();
  path
}

/// The nodes from `from` to `to`, the shortest way when `wander` is 0, or else a loop-erased random walk biased
/// toward `to`. Each step's chance falls off exponentially with the detour it adds, over `wander²` average edge
/// lengths. Falls back to the shortest path if the walk runs too long. Empty if `to` can't be reached.
fn wandering_path(graph: &Graph, from: usize, to: usize, wander: f32, seed: impl std::hash::Hash + Copy) -> Vec<usize> {
  let (_, previous) = shortest_paths(graph, from, |_, _, length| length);
  let shortest = path_to(&previous, from, to);
  if wander <= 0.0 || shortest.is_empty() {
    return shortest;
  }
  let (remaining, _) = shortest_paths(graph, to, |_, _, length| length);
  let lengths: Vec<f32> = (0..graph.nodes().len()).flat_map(|node| graph.neighbors(node).iter().map(|&(_, length)| length)).collect();
  let scale = wander * wander * lengths.iter().sum::<f32>() / lengths.len().max(1) as f32;

  let mut path = vec![from];
  // where each node is on `path`, to cut out a loop when the walk comes back to it
  let mut position = vec![None; graph.nodes().len()];
  position[from] = Some(0);
  for step in 0..MAX_WALK_STEPS {
    let node = *path.last().unwrap_or(&from);
    if node == to {
      return path;
    }
    let weights: Vec<(usize, f32)> = graph
      .neighbors(node)
      .iter()
      .filter(|&&(next, _)| remaining[next].is_finite())
      .map(|&(next, length)| (next, (-(length + remaining[next] - remaining[node]).max(0.0) / scale).exp()))
      .collect();
    let mut pick = rnd((seed, step))[0] * weights.iter().map(|&(_, weight)| weight).sum::<f32>();
    let Some(&(next, _)) = weights.iter().find(|&&(_, weight)| {
      pick -= weight;
      pick < 0.0
    }).or(weights.last()) else {
      break;
    };
    match position[next] {
      Some(at) => {
        path.drain(at + 1..).for_each(|cut| position[cut] = None);
      }
      None => {
        position[next] = Some(path.len());
        path.push(next);
      }
    }
  }
  shortest
}

/// The order an A* search settles nodes in, guided by straight-line distance to `to`, ending when it settles `to`
fn a_star_order(graph: &Graph, from: usize, to: Option<usize>) -> Vec<usize> {
  let target = to.map(|to| graph.nodes()[to].pos);
  let estimate = |node: usize| target.map_or(0.0, |target| graph.nodes()[node].pos.distance(target));
  let mut distances = vec![f32::INFINITY; graph.nodes().len()];
  let mut settled = vec![false; graph.nodes().len()];
  let mut order = Vec::new();
  let mut queue = BinaryHeap::from([Queued { cost: estimate(from), node: from }]);
  distances[from] = 0.0;
  while let Some(Queued { node, .. }) = queue.pop() {
    if std::mem::replace(&mut settled[node], true) {
      continue;
    }
    order.push(node);
    if Some(node) == to {
      break;
    }
    for &(next, length) in graph.neighbors(node) {
      let through = distances[node] + length;
      if through < distances[next] {
        distances[next] = through;
        queue.push(Queued { cost: through + estimate(next), node: next });
      }
    }
  }
  order
}

/// The nodes a random walk is on for its first `steps` steps after `from`, with `from` first
fn random_walk(graph: &Graph, from: usize, steps: usize, seed: impl std::hash::Hash + Copy) -> Vec<usize> {
  let mut path = vec![from];
  let mut came_from = None;
  for step in 0..steps {
    let node = *path.last().unwrap_or(&from);
    let options: Vec<usize> = graph.neighbors(node).iter().map(|(next, _)| *next).filter(|&next| Some(next) != came_from).collect();
    let options = if options.is_empty() { graph.neighbors(node).iter().map(|(next, _)| *next).collect() } else { options };
    let Some(&next) = options.get((rnd((seed, step))[0] * options.len() as f32) as usize % options.len().max(1)) else {
      break;
    };
    came_from = Some(node);
    path.push(next);
  }
  path
}

/// A node waiting in a priority queue, popped lowest cost first, then lowest index
#[derive(PartialEq)]
struct Queued {
  cost: f32,
  node: usize,
}

impl Eq for Queued {}

impl Ord for Queued {
  fn cmp(&self, other: &Self) -> Ordering {
    other.cost.total_cmp(&self.cost).then(other.node.cmp(&self.node))
  }
}

impl PartialOrd for Queued {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::animation::Cycle;
  use crate::scene::value::tests::env;
  use crate::scene::{GraphNode, Params, Timing};

  /// Nodes "0" to "n-1" in a row, one apart
  fn row(n: usize) -> Graph {
    Graph::build((0..n).map(|i| GraphNode::new(i.to_string(), Vec2::new(i as f32, 0.0))).collect())
  }

  /// A 3 by 3 grid, named by index, row by row
  fn grid() -> Graph {
    let nodes = (0..9).map(|i| GraphNode::new(i.to_string(), Vec2::new((i % 3) as f32, (i / 3) as f32)));
    Graph::build(nodes.collect())
  }

  /// From node "0", two steps a second, in a cycle `duration` seconds long
  fn traverse(graph: &Graph, algorithm: TraversalAlgorithm, duration: f32) -> TraversalVisits {
    let params = Params::new();
    let env = Env { timing: Some(Timing::new(duration, Cycle::Forever)), ..env(0.0, &params) };
    Traversal::new("t", NodeRef::name("0"), algorithm, 2.0).visits(graph, &env)
  }

  fn visits(graph: &Graph, algorithm: TraversalAlgorithm, duration: f32) -> Vec<Vec<f32>> {
    traverse(graph, algorithm, duration).by_node
  }

  /// When each node is first visited, `None` for never
  fn first(visits: &[Vec<f32>]) -> Vec<Option<f32>> {
    visits.iter().map(|times| times.first().copied()).collect()
  }

  #[test]
  fn breadth_first_visits_by_edge_count() {
    let visits = visits(&grid(), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, 0.0);
    // the middle is one diagonal from the corner
    assert_eq!(first(&visits), [0.0, 0.5, 1.0, 0.5, 0.5, 1.0, 1.0, 1.0, 1.0].map(Some));
  }

  #[test]
  fn dijkstra_visits_by_distance() {
    let nodes = [0.0, 1.0, 4.0].map(|x| GraphNode::new(format!("{x}"), Vec2::new(x, 0.0))).to_vec();
    let graph = Graph::build(nodes);
    let params = Params::new();
    let traversal = Traversal::new("t", NodeRef::name("0"), TraversalAlgorithm::Dijkstra { wander: 0.0.into() }, 2.0).start(1.0);
    assert_eq!(first(&traversal.visits(&graph, &env(0.0, &params)).by_node), [Some(1.0), Some(1.5), Some(3.0)]);
  }


  #[test]
  fn a_duration_stretches_the_traversal_to_fit() {
    let params = Params::new();
    let env = Env { timing: Some(Timing::new(7.0, Cycle::Forever)), ..env(0.0, &params) };
    let traversal = |speed: Speed| Traversal::new("t", NodeRef::name("0"), TraversalAlgorithm::Dijkstra { wander: 0.0.into() }, speed).start(1.0);
    // 2 units to the end plus a step there, in 3 seconds or the 6 left in the cycle
    let fixed = traversal(Speed::Duration(Some(3.0))).visits(&row(3), &env);
    assert_eq!((fixed.rate, first(&fixed.by_node)[2]), (1.0, Some(3.0)));
    let rest = traversal(Speed::Duration(None)).visits(&row(3), &env);
    assert_eq!(first(&rest.by_node)[2], Some(5.0));
  }

  #[test]
  fn a_shortest_path_over_the_rest_of_the_cycle_reaches_its_target_before_the_cycle_wraps() {
    let params = Params::new();
    let env = Env { timing: Some(Timing::new(10.0, Cycle::Forever)), ..env(0.0, &params) };
    let to = TraversalAlgorithm::ShortestPath { to: NodeRef::name("4"), wander: 0.0.into() };
    let path = Traversal::new("t", NodeRef::name("0"), to, Speed::Duration(None)).visits(&row(5), &env);
    // 4 edges and a step at the target, 2 seconds each
    assert_eq!(first(&path.by_node), [0.0, 2.0, 4.0, 6.0, 8.0].map(Some));
  }

  #[test]
  fn a_random_walk_with_a_duration_takes_a_step_per_node() {
    let params = Params::new();
    let walk = Traversal::new("t", NodeRef::name("0"), TraversalAlgorithm::RandomWalk, Speed::Duration(Some(10.0))).visits(&grid(), &env(0.0, &params));
    assert_eq!(walk.by_node.iter().map(Vec::len).sum::<usize>(), 10);
    assert_eq!(walk.rate, 1.0);
  }
  #[test]
  fn depth_first_visits_every_node_once_one_per_step() {
    let visits = visits(&grid(), TraversalAlgorithm::DepthFirst, 0.0);
    let mut times: Vec<f32> = visits.iter().map(|times| {
      assert_eq!(times.len(), 1);
      times[0]
    }).collect();
    times.sort_by(f32::total_cmp);
    assert_eq!(times, (0..9).map(|step| step as f32 / 2.0).collect::<Vec<_>>());
  }

  #[test]
  fn a_shortest_path_only_visits_the_path() {
    let visits = visits(&row(4), TraversalAlgorithm::ShortestPath { to: NodeRef::name("2"), wander: 0.0.into() }, 0.0);
    assert_eq!(first(&visits), [Some(0.0), Some(0.5), Some(1.0), None]);
  }

  #[test]
  fn a_shortest_path_to_somewhere_unreachable_visits_nothing() {
    let visits = visits(&row(3), TraversalAlgorithm::ShortestPath { to: NodeRef::name("nope"), wander: 0.0.into() }, 0.0);
    assert!(visits.iter().all(Vec::is_empty));
  }

  #[test]
  fn wander_wanders_off_the_shortest_path_and_still_reaches_the_end() {
    let grid = Graph::build((0..144).map(|i| GraphNode::new(i.to_string(), Vec2::new((i % 12) as f32, (i / 12) as f32))).collect());
    let params = Params::new();
    let env = Env { timing: Some(Timing::new(10.0, Cycle::Forever)), ..env(0.0, &params) };
    let path = |name: &str, wander: f32| {
      let to = TraversalAlgorithm::ShortestPath { to: NodeRef::name("23"), wander: wander.into() };
      Traversal::new(name, NodeRef::name("0"), to, 2.0).visits(&grid, &env).by_node
    };
    let lengths: Vec<usize> = (0..10).map(|seed| path(&seed.to_string(), 3.0)).inspect(|path| assert!(!path[23].is_empty())).map(|path| {
      // a path with its loops cut out lights each node once
      assert!(path.iter().all(|times| times.len() <= 1));
      path.iter().filter(|times| !times.is_empty()).count()
    }).collect();
    let straight = path("t", 0.0).iter().filter(|times| !times.is_empty()).count();
    assert!(lengths.iter().sum::<usize>() > 15 * straight, "{lengths:?} against {straight}");
  }

  #[test]
  fn a_wandering_fill_reaches_everything_in_a_new_order_and_ends_on_time() {
    let grid = Graph::build((0..144).map(|i| GraphNode::new(i.to_string(), Vec2::new((i % 12) as f32, (i / 12) as f32))).collect());
    let params = Params::new();
    let fill = |by_steps: bool, wander: f32| {
      let algorithm = if by_steps { TraversalAlgorithm::BreadthFirst { wander: wander.into() } } else { TraversalAlgorithm::Dijkstra { wander: wander.into() } };
      Traversal::new("t", NodeRef::name("0"), algorithm, 2.0).visits(&grid, &env(0.0, &params)).by_node.iter().map(|times| times[0]).collect::<Vec<f32>>()
    };
    for by_steps in [true, false] {
      let (even, wandering) = (fill(by_steps, 0.0), fill(by_steps, 3.0));
      assert_eq!(wandering.iter().copied().fold(0.0, f32::max), even.iter().copied().fold(0.0, f32::max));
      assert!(even.iter().zip(&wandering).filter(|(even, wandering)| even != wandering).count() > 100);
    }
  }

  #[test]
  fn wander_still_reaches_the_end() {
    let visits = visits(&row(5), TraversalAlgorithm::ShortestPath { to: NodeRef::name("4"), wander: 2.0.into() }, 0.0);
    assert_eq!(first(&visits), [0.0, 0.5, 1.0, 1.5, 2.0].map(Some));
  }

  #[test]
  fn a_fan_out_goes_to_the_target_and_spreads_across_the_arc() {
    let grid = Graph::build((0..441).map(|i| GraphNode::new(i.to_string(), Vec2::new((i % 21) as f32, (i / 21) as f32))).collect());
    let at = |node: usize| Vec2::new((node % 21) as f32, (node / 21) as f32);
    let params = Params::new();
    let fan = |count: f32| {
      let algorithm = TraversalAlgorithm::FanOut { to: NodeRef::name("430"), angle: 90.0.into(), count: count.into(), wander: 0.0.into() };
      // from the middle of the bottom row to the middle of the top
      Traversal::new("t", NodeRef::name("10"), algorithm, 2.0).visits(&grid, &env(0.0, &params)).by_node
    };
    let visited = |visits: &[Vec<f32>]| visits.iter().filter(|times| !times.is_empty()).count();
    let straight = TraversalAlgorithm::ShortestPath { to: NodeRef::name("430"), wander: 0.0.into() };
    let straight = Traversal::new("t", NodeRef::name("10"), straight, 2.0).visits(&grid, &env(0.0, &params)).by_node;
    assert_eq!(visited(&fan(0.0)), visited(&straight));

    let spread = fan(4.0);
    assert!(!spread[430].is_empty());
    // each node is visited once, the start the paths share included
    assert!(spread.iter().all(|times| times.len() <= 1));
    assert_eq!(spread[10], [0.0]);
    // the far ends are within 45° either side of straight up, and some are well off to each side
    let ends: Vec<Vec2> = (0..441).filter(|&node| spread[node].first().is_some_and(|&at| at >= 9.0)).map(at).collect();
    assert!(ends.iter().all(|&end| (end - at(10)).angle_to(Vec2::Y).abs() <= 46f32.to_radians()), "{ends:?}");
    assert!(ends.iter().any(|end| end.x < 6.0) && ends.iter().any(|end| end.x > 14.0), "{ends:?}");
  }

  #[test]
  fn a_star_stops_at_its_goal() {
    let visits = visits(&row(5), TraversalAlgorithm::AStar { to: NodeRef::name("2") }, 0.0);
    assert_eq!(first(&visits), [Some(0.0), Some(0.5), Some(1.0), None, None]);
  }

  #[test]
  fn a_random_walk_takes_a_step_at_each_rate_tick_for_the_whole_cycle() {
    let visits = visits(&grid(), TraversalAlgorithm::RandomWalk, 2.25);
    let mut times: Vec<f32> = visits.into_iter().flatten().collect();
    times.sort_by(f32::total_cmp);
    assert_eq!(times, [0.0, 0.5, 1.0, 1.5, 2.0]);
  }

  #[test]
  fn a_random_walk_only_turns_back_at_a_dead_end() {
    let visits = visits(&row(3), TraversalAlgorithm::RandomWalk, 2.0);
    // 0 → 1 → 2 → 1 → 0: from the middle, it can't go back the way it came
    assert_eq!(visits[0], [0.0, 2.0]);
    assert_eq!(visits[1], [0.5, 1.5]);
    assert_eq!(visits[2], [1.0]);
  }

  #[test]
  fn a_random_walk_differs_by_cycle() {
    let params = Params::new();
    let traversal = Traversal::new("t", NodeRef::name("4"), TraversalAlgorithm::RandomWalk, 10.0);
    let walk = |cycle| traversal.visits(&grid(), &Env { cycle, ..env(5.0, &params) }).by_node;
    assert_ne!(walk(0), walk(1));
    assert_eq!(walk(0), walk(0));
  }

  #[test]
  fn a_missing_start_or_no_rate_visits_nothing() {
    let params = Params::new();
    let graph = row(3);
    let nowhere = Traversal::new("t", NodeRef::name("nope"), TraversalAlgorithm::Dijkstra { wander: 0.0.into() }, 1.0);
    let stopped = Traversal::new("t", NodeRef::name("0"), TraversalAlgorithm::Dijkstra { wander: 0.0.into() }, 0.0);
    assert!(nowhere.visits(&graph, &env(0.0, &params)).by_node.iter().all(Vec::is_empty));
    assert!(stopped.visits(&graph, &env(0.0, &params)).by_node.iter().all(Vec::is_empty));
  }

  #[test]
  fn nearest_starts_from_the_closest_node() {
    let params = Params::new();
    let traversal = Traversal::new("t", NodeRef::Nearest(Vec2::new(2.2, 0.0).into()), TraversalAlgorithm::Dijkstra { wander: 0.0.into() }, 1.0);
    assert_eq!(first(&traversal.visits(&row(3), &env(0.0, &params)).by_node), [Some(2.0), Some(1.0), Some(0.0)]);
  }

  #[test]
  fn it_ends_at_the_node_it_visits_last() {
    let path = traverse(&grid(), TraversalAlgorithm::ShortestPath { to: NodeRef::name("5"), wander: 0.0.into() }, 10.0);
    assert_eq!(path.ends, Some((Vec2::ZERO, Vec2::new(2.0, 1.0))));
    let spread = traverse(&grid(), TraversalAlgorithm::Dijkstra { wander: 0.0.into() }, 10.0);
    assert_eq!(spread.ends, Some((Vec2::ZERO, Vec2::new(2.0, 2.0))));
    let walk = traverse(&row(3), TraversalAlgorithm::RandomWalk, 2.0);
    assert_eq!(walk.ends, Some((Vec2::ZERO, Vec2::ZERO)));
    assert_eq!(traverse(&grid(), TraversalAlgorithm::ShortestPath { to: NodeRef::name("nope"), wander: 0.0.into() }, 10.0).ends, None);
  }
}
