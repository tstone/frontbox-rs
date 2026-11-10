use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{Graph, GraphNode, Sample};

/// How far an LED can be off a plane, in the plane's units, and still be a node of a graph on it. LEDs on ramps and other
/// raised parts above it are left out.
pub const GRAPH_PLANE_TOLERANCE: f32 = 0.5;

/// Slack on [`GRAPH_PLANE_TOLERANCE`] for rounding in the plane's transform, so LEDs placed right at it are all kept
const ROUNDING: f32 = 1e-3;

/// Graphs already built, as each playback places its layers anew but the LEDs never move
static GRAPHS: LazyLock<Mutex<HashMap<GraphKey, Arc<Graph>>>> = LazyLock::new(Default::default);

/// Narrows a layer's placed LEDs, from [`place_leds`](crate::place_leds), to the nodes of its graph, and builds the graph
/// from where they sample it. Each node is named for its LED.
///
/// On a plane, LEDs more than [`GRAPH_PLANE_TOLERANCE`] off it are left out. Each kept sample is numbered as its node.
pub fn place_graph<'a>(placed: Vec<(&'a LED, Sample)>, plane: Option<&ReferencePlane>) -> (Vec<(&'a LED, Sample)>, Arc<Graph>) {
  let on_plane = |led: &LED| match (plane, led.location) {
    (Some(plane), Some(location)) => plane.to_local(location).z.abs() <= GRAPH_PLANE_TOLERANCE + ROUNDING,
    _ => true,
  };
  let placed: Vec<_> = placed.into_iter().filter(|(led, _)| on_plane(led)).enumerate().map(|(node, (led, at))| (led, at.at_node(node))).collect();
  let nodes: Vec<GraphNode> = placed.iter().map(|(led, at)| GraphNode::new(led.name.clone(), at.pos)).collect();

  let key = GraphKey::new(&nodes);
  let mut graphs = GRAPHS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
  let graph = graphs.entry(key).or_insert_with(|| Arc::new(Graph::build(nodes))).clone();
  (placed, graph)
}

/// Everything a graph is built from: its nodes, with positions by their bits so they can be hashed
#[derive(PartialEq, Eq, Hash)]
struct GraphKey {
  nodes: Vec<(String, [u32; 2])>,
}

impl GraphKey {
  fn new(nodes: &[GraphNode]) -> Self {
    Self { nodes: nodes.iter().map(|node| (node.name.clone(), node.pos.to_array().map(f32::to_bits))).collect() }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn leds_right_at_the_tolerance_are_all_kept_on_a_tilted_plane() {
    let plane = ReferencePlane::new("Tilted").origin(Vec3::new(2.0, 3.0, 4.0)).rotation(Quat::from_axis_angle(Vec3::X, 6.5f32.to_radians())).build();
    // a ring of 12 set just under the plane, as inserts under a playfield are
    let leds: Vec<LED> = (0..12)
      .map(|i| {
        let angle = i as f32 / 12.0 * std::f32::consts::TAU;
        let local = Vec3::new(13.869 + 0.75 * angle.cos(), 18.161 + 0.75 * angle.sin(), -GRAPH_PLANE_TOLERANCE);
        LED { name: format!("led{i}"), location: Some(plane.to_absolute(local)), ..Default::default() }
      })
      .collect();
    let placed = leds.iter().map(|led| (led, Sample::new(Vec2::ZERO, 0.1))).collect();
    let (kept, _) = place_graph(placed, Some(&plane));
    assert_eq!(kept.len(), 12);
  }
}
