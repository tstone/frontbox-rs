use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{Graph, GraphNode, Sample};

/// How far a light can be off a plane, in the plane's units, and still be a node of a graph on it. Lights on ramps and other
/// raised parts above it are left out.
pub const GRAPH_PLANE_TOLERANCE: f32 = 0.5;

/// Slack on [`GRAPH_PLANE_TOLERANCE`] for rounding in the plane's transform, so lights placed right at it are all kept
const ROUNDING: f32 = 1e-3;

/// How many graphs are kept. A game has one for each set of lights its scenes draw graphs on; moving a light (in the
/// console) makes a new one for each set it's in, which is what pushes old ones out.
const GRAPHS_KEPT: usize = 64;

/// Graphs already built, as each playback places its layers anew but the lights never move
static GRAPHS: LazyLock<Mutex<GraphCache>> = LazyLock::new(Default::default);

/// Narrows a layer's placed lights, from [`place_lights`](crate::place_lights), to the nodes of its graph, and builds the graph
/// from where they sample it. Each node is named for its light.
///
/// On a plane, lights more than [`GRAPH_PLANE_TOLERANCE`] off it are left out. Each kept sample is numbered as its node.
pub fn place_graph<'a>(
  placed: Vec<(&'a Light, Sample)>,
  plane: Option<&ReferencePlane>,
) -> (Vec<(&'a Light, Sample)>, Arc<Graph>) {
  let on_plane = |light: &Light| match (plane, light.location) {
    (Some(plane), Some(location)) => {
      plane.to_local(location).z.abs() <= GRAPH_PLANE_TOLERANCE + ROUNDING
    }
    _ => true,
  };
  let placed: Vec<_> = placed
    .into_iter()
    .filter(|(light, _)| on_plane(light))
    .enumerate()
    .map(|(node, (light, at))| (light, at.at_node(node)))
    .collect();
  let nodes: Vec<GraphNode> = placed
    .iter()
    .map(|(light, at)| GraphNode::new(light.name.clone(), at.pos))
    .collect();

  let key = GraphKey::new(&nodes);
  let mut graphs = GRAPHS
    .lock()
    .unwrap_or_else(|poisoned| poisoned.into_inner());
  let graph = graphs.get_or_build(key, || Graph::build(nodes), GRAPHS_KEPT);
  (placed, graph)
}

/// Graphs by what they're built from, dropping the ones used longest ago once there are more than it's asked to keep
#[derive(Default)]
struct GraphCache {
  /// Each graph, and the `uses` count when it was last asked for
  graphs: HashMap<GraphKey, (Arc<Graph>, u64)>,
  uses: u64,
}

impl GraphCache {
  fn get_or_build(
    &mut self,
    key: GraphKey,
    build: impl FnOnce() -> Graph,
    keep: usize,
  ) -> Arc<Graph> {
    self.uses += 1;
    let uses = self.uses;
    let (graph, used) = self
      .graphs
      .entry(key)
      .or_insert_with(|| (Arc::new(build()), uses));
    *used = uses;
    let graph = graph.clone();
    while self.graphs.len() > keep {
      let oldest = self.graphs.values().map(|(_, used)| *used).min();
      self.graphs.retain(|_, (_, used)| Some(*used) != oldest);
    }
    graph
  }
}

/// Everything a graph is built from: its nodes, with positions by their bits so they can be hashed
#[derive(PartialEq, Eq, Hash)]
struct GraphKey {
  nodes: Vec<(String, [u32; 2])>,
}

impl GraphKey {
  fn new(nodes: &[GraphNode]) -> Self {
    Self {
      nodes: nodes
        .iter()
        .map(|node| (node.name.clone(), node.pos.to_array().map(f32::to_bits)))
        .collect(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_cache_drops_the_graph_used_longest_ago() {
    let key = |name: &str| GraphKey::new(&[GraphNode::new(name.to_string(), Vec2::ZERO)]);
    let build = || Graph::build(Vec::new());
    let mut cache = GraphCache::default();
    let a = cache.get_or_build(key("a"), build, 2);
    cache.get_or_build(key("b"), build, 2);
    let again = cache.get_or_build(key("a"), || panic!("a is already built"), 2);
    assert!(Arc::ptr_eq(&a, &again));

    cache.get_or_build(key("c"), build, 2);

    assert_eq!(cache.graphs.len(), 2);
    assert!(cache.graphs.contains_key(&key("a")));
    assert!(!cache.graphs.contains_key(&key("b")));
  }

  #[test]
  fn lights_right_at_the_tolerance_are_all_kept_on_a_tilted_plane() {
    let plane = ReferencePlane::new("Tilted")
      .origin(Vec3::new(2.0, 3.0, 4.0))
      .rotation(Quat::from_axis_angle(Vec3::X, 6.5f32.to_radians()))
      .build();
    // a ring of 12 set just under the plane, as inserts under a playfield are
    let lights: Vec<Light> = (0..12)
      .map(|i| {
        let angle = i as f32 / 12.0 * std::f32::consts::TAU;
        let local = Vec3::new(
          13.869 + 0.75 * angle.cos(),
          18.161 + 0.75 * angle.sin(),
          -GRAPH_PLANE_TOLERANCE,
        );
        Light {
          name: format!("light{i}"),
          location: Some(plane.to_absolute(local)),
          ..Default::default()
        }
      })
      .collect();
    let placed = lights
      .iter()
      .map(|light| (light, Sample::new(Vec2::ZERO, 0.1)))
      .collect();
    let (kept, _) = place_graph(placed, Some(&plane));
    assert_eq!(kept.len(), 12);
  }
}
