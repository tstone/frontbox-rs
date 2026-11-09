use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{EdgeRule, Edges, Graph, GraphNode, Sample};

/// How far an LED can be off a plane, in the plane's units, and still be a node of a graph on it. LEDs on ramps and other
/// raised parts above it are left out.
pub const GRAPH_PLANE_TOLERANCE: f32 = 0.5;

/// Graphs already built, as each playback places its layers anew but the LEDs never move
static GRAPHS: LazyLock<Mutex<HashMap<GraphKey, Arc<Graph>>>> = LazyLock::new(Default::default);

/// Narrows a layer's placed LEDs, from [`place_leds`](crate::place_leds), to the nodes of its graph, and builds the graph
/// from where they sample it. Each node is named for its LED.
///
/// On a plane, LEDs more than [`GRAPH_PLANE_TOLERANCE`] off it are left out. Each kept sample is numbered as its node.
pub fn place_graph<'a>(placed: Vec<(&'a LED, Sample)>, plane: Option<&ReferencePlane>, edges: &Edges) -> (Vec<(&'a LED, Sample)>, Arc<Graph>) {
  let on_plane = |led: &LED| match (plane, led.location) {
    (Some(plane), Some(location)) => plane.to_local(location).z.abs() <= GRAPH_PLANE_TOLERANCE,
    _ => true,
  };
  let placed: Vec<_> = placed.into_iter().filter(|(led, _)| on_plane(led)).enumerate().map(|(node, (led, at))| (led, at.at_node(node))).collect();
  let nodes: Vec<GraphNode> = placed.iter().map(|(led, at)| GraphNode::new(led.name.clone(), at.pos)).collect();

  let key = GraphKey::new(&nodes, edges);
  let mut graphs = GRAPHS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let graph = graphs.entry(key).or_insert_with(|| Arc::new(Graph::build(nodes, edges))).clone();
  (placed, graph)
}

/// Everything a graph is built from, with positions and lengths by their bits so they can be hashed
#[derive(PartialEq, Eq, Hash)]
struct GraphKey {
  nodes: Vec<(String, [u32; 2])>,
  rule: EdgeRule,
  max_length: Option<u32>,
}

impl GraphKey {
  fn new(nodes: &[GraphNode], edges: &Edges) -> Self {
    Self {
      nodes: nodes.iter().map(|node| (node.name.clone(), node.pos.to_array().map(f32::to_bits))).collect(),
      rule: edges.rule,
      max_length: edges.max_length.map(f32::to_bits),
    }
  }
}
