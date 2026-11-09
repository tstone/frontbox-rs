use std::collections::{BinaryHeap, VecDeque};
use std::cmp::Ordering;

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::scene::{Edges, Env, Graph, Prop, TraversalVisits, rnd};

/// Most steps a random walk takes in one frame, so a huge rate can't stall playback
const MAX_WALK_STEPS: usize = 100_000;

/// Limits a layer to the nodes of a graph, and runs traversals over it whose visits [`Prop::Visit`] responds to
///
/// The graph is built by the device from where its points are, with [`Graph::build`] and these `edges`. Samples that
/// aren't a node of it are left out of the layer.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GraphMask {
  #[serde(default)]
  pub edges: Edges,
  #[serde(default)]
  pub traversals: Vec<Traversal>,
}

impl GraphMask {
  pub fn new(traversals: Vec<Traversal>) -> Self {
    Self { edges: Edges::default(), traversals }
  }

  pub fn edges(mut self, edges: Edges) -> Self {
    self.edges = edges;
    self
  }

  pub(crate) fn is_animated(&self) -> bool {
    !self.traversals.is_empty()
  }
}

/// A walk over a graph, visiting nodes at a set rate, starting from one node
///
/// Visits are worked out fresh each cycle, so random choices differ by cycle and playback, as with [`Random`](crate::scene::Random).
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traversal {
  /// How a [`VisitResponse`](crate::scene::VisitResponse) refers to it. Unique within the layer.
  pub name: String,
  pub from: NodeRef,
  pub algorithm: TraversalAlgorithm,
  /// Steps per second, or for algorithms that go by distance, scene units per second. A step is also the unit of a
  /// response's timing. Nothing is visited unless it's above 0.
  pub rate: Prop<f32>,
  /// Seconds into the cycle when it leaves `from`
  #[serde(default)]
  pub start: f32,
}

impl Traversal {
  pub fn new(name: impl Into<String>, from: NodeRef, algorithm: TraversalAlgorithm, rate: impl Into<Prop<f32>>) -> Self {
    Self { name: name.into(), from, algorithm, rate: rate.into(), start: 0.0 }
  }

  pub fn start(mut self, start: f32) -> Self {
    self.start = start;
    self
  }

  /// Every visit it makes this cycle, including ones still to come
  pub(crate) fn visits(&self, graph: &Graph, env: &Env) -> TraversalVisits {
    let rate = self.rate.resolve(env);
    let mut by_node = vec![Vec::new(); graph.nodes().len()];
    let Some(from) = self.from.resolve(graph, env).filter(|_| rate > 0.0) else {
      return TraversalVisits { name: self.name.clone(), rate, by_node, ends: None };
    };
    let visits = &mut by_node;

    let seed = (&self.name, env.playback_seed, env.cycle);
    let mut visit = |node: usize, steps: f32| visits[node].push(self.start + steps / rate);
    match &self.algorithm {
      TraversalAlgorithm::BreadthFirst => {
        for (node, hops) in hops(graph, from).into_iter().enumerate() {
          if let Some(hops) = hops {
            visit(node, hops as f32);
          }
        }
      }
      TraversalAlgorithm::DepthFirst => {
        for (step, node) in depth_first(graph, from, seed).into_iter().enumerate() {
          visit(node, step as f32);
        }
      }
      TraversalAlgorithm::Dijkstra => {
        let (distances, _) = shortest_paths(graph, from, |_, _, length| length);
        for (node, distance) in distances.into_iter().enumerate().filter(|(_, distance)| distance.is_finite()) {
          visit(node, distance);
        }
      }
      TraversalAlgorithm::ShortestPath { to, jitter } => {
        let jittered = |a: usize, b: usize, length: f32| length * (1.0 + jitter * rnd((seed, a.min(b), a.max(b)))[0]);
        let (_, previous) = shortest_paths(graph, from, jittered);
        let path = to.resolve(graph, env).map_or(Vec::new(), |to| path_to(&previous, from, to));
        let mut distance = 0.0;
        for pair in path.windows(2) {
          distance += graph.nodes()[pair[0]].pos.distance(graph.nodes()[pair[1]].pos);
          visit(pair[1], distance);
        }
        if let Some(&node) = path.first() {
          visit(node, 0.0);
        }
      }
      TraversalAlgorithm::AStar { to } => {
        let to = to.resolve(graph, env);
        for (step, node) in a_star_order(graph, from, to).into_iter().enumerate() {
          visit(node, step as f32);
        }
      }
      TraversalAlgorithm::RandomWalk => {
        let cycle_end = env.timing.map_or(env.time, |timing| timing.duration);
        let steps = ((cycle_end - self.start) * rate).floor();
        if steps >= 0.0 {
          for (step, node) in random_walk(graph, from, (steps as usize).min(MAX_WALK_STEPS), seed).into_iter().enumerate() {
            visit(node, step as f32);
          }
        }
      }
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

/// Which nodes a traversal visits, in what order, and how far apart in time
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TraversalAlgorithm {
  /// Every reachable node, by the number of edges from the start: a step is one edge, so nodes the same number of edges
  /// away are visited together
  BreadthFirst,
  /// Every reachable node, one per step, following one branch to its end before backing up to the next. Branches are
  /// taken in a random order.
  DepthFirst,
  /// Every reachable node, by path distance from the start. Goes by distance.
  Dijkstra,
  /// Only the nodes on the shortest path to `to`. Goes by distance. Nothing is visited if `to` can't be reached.
  ShortestPath {
    to: NodeRef,
    /// Lengthens each edge by up to this fraction of itself, at random, when choosing the path, so it wanders off the
    /// straightest line. 0 always takes the shortest.
    #[serde(default)]
    jitter: f32,
  },
  /// The nodes an A* search for `to` explores, one per step in the order it settles them, ending at `to`. Without a
  /// reachable `to`, it explores everything reachable.
  AStar { to: NodeRef },
  /// One random neighbor per step, not going straight back unless it's at a dead end. Nodes can be visited many times.
  /// The whole cycle's walk is chosen at its start, so it has a known end.
  RandomWalk,
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

/// The number of edges from `from` to each node. `None` where it can't be reached.
fn hops(graph: &Graph, from: usize) -> Vec<Option<usize>> {
  let mut hops = vec![None; graph.nodes().len()];
  hops[from] = Some(0);
  let mut queue = VecDeque::from([from]);
  while let Some(node) = queue.pop_front() {
    for &(next, _) in graph.neighbors(node) {
      if hops[next].is_none() {
        hops[next] = Some(hops[node].unwrap_or(0) + 1);
        queue.push_back(next);
      }
    }
  }
  hops
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
    Graph::build((0..n).map(|i| GraphNode::new(i.to_string(), Vec2::new(i as f32, 0.0))).collect(), &Edges::default())
  }

  /// A 3 by 3 grid, named by index, row by row
  fn grid() -> Graph {
    let nodes = (0..9).map(|i| GraphNode::new(i.to_string(), Vec2::new((i % 3) as f32, (i / 3) as f32)));
    Graph::build(nodes.collect(), &Edges::default())
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
    let visits = visits(&grid(), TraversalAlgorithm::BreadthFirst, 0.0);
    assert_eq!(first(&visits), [0.0, 0.5, 1.0, 0.5, 1.0, 1.5, 1.0, 1.5, 2.0].map(Some));
  }

  #[test]
  fn dijkstra_visits_by_distance() {
    let nodes = [0.0, 1.0, 4.0].map(|x| GraphNode::new(format!("{x}"), Vec2::new(x, 0.0))).to_vec();
    let graph = Graph::build(nodes, &Edges::default());
    let params = Params::new();
    let traversal = Traversal::new("t", NodeRef::name("0"), TraversalAlgorithm::Dijkstra, 2.0).start(1.0);
    assert_eq!(first(&traversal.visits(&graph, &env(0.0, &params)).by_node), [Some(1.0), Some(1.5), Some(3.0)]);
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
    let visits = visits(&row(4), TraversalAlgorithm::ShortestPath { to: NodeRef::name("2"), jitter: 0.0 }, 0.0);
    assert_eq!(first(&visits), [Some(0.0), Some(0.5), Some(1.0), None]);
  }

  #[test]
  fn a_shortest_path_to_somewhere_unreachable_visits_nothing() {
    let visits = visits(&row(3), TraversalAlgorithm::ShortestPath { to: NodeRef::name("nope"), jitter: 0.0 }, 0.0);
    assert!(visits.iter().all(Vec::is_empty));
  }

  #[test]
  fn jitter_still_reaches_the_end() {
    let visits = visits(&grid(), TraversalAlgorithm::ShortestPath { to: NodeRef::name("8"), jitter: 2.0 }, 0.0);
    assert_eq!(visits.iter().filter(|times| !times.is_empty()).count(), 5);
    assert_eq!(first(&visits)[8], Some(2.0));
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
    let nowhere = Traversal::new("t", NodeRef::name("nope"), TraversalAlgorithm::BreadthFirst, 1.0);
    let stopped = Traversal::new("t", NodeRef::name("0"), TraversalAlgorithm::BreadthFirst, 0.0);
    assert!(nowhere.visits(&graph, &env(0.0, &params)).by_node.iter().all(Vec::is_empty));
    assert!(stopped.visits(&graph, &env(0.0, &params)).by_node.iter().all(Vec::is_empty));
  }

  #[test]
  fn nearest_starts_from_the_closest_node() {
    let params = Params::new();
    let traversal = Traversal::new("t", NodeRef::Nearest(Vec2::new(2.2, 0.0).into()), TraversalAlgorithm::BreadthFirst, 1.0);
    assert_eq!(first(&traversal.visits(&row(3), &env(0.0, &params)).by_node), [Some(2.0), Some(1.0), Some(0.0)]);
  }

  #[test]
  fn it_ends_at_the_node_it_visits_last() {
    let path = traverse(&grid(), TraversalAlgorithm::ShortestPath { to: NodeRef::name("5"), jitter: 0.0 }, 10.0);
    assert_eq!(path.ends, Some((Vec2::ZERO, Vec2::new(2.0, 1.0))));
    let spread = traverse(&grid(), TraversalAlgorithm::BreadthFirst, 10.0);
    assert_eq!(spread.ends, Some((Vec2::ZERO, Vec2::new(2.0, 2.0))));
    let walk = traverse(&row(3), TraversalAlgorithm::RandomWalk, 2.0);
    assert_eq!(walk.ends, Some((Vec2::ZERO, Vec2::ZERO)));
    assert_eq!(traverse(&grid(), TraversalAlgorithm::ShortestPath { to: NodeRef::name("nope"), jitter: 0.0 }, 10.0).ends, None);
  }
}
