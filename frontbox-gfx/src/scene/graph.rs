use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Points joined by weighted edges, for a layer's [`GraphMask`](crate::scene::GraphMask) to run traversals over
///
/// Like sample positions, a graph comes from the device rather than the scene: the caller builds it from where its points
/// are, using the layer's [`Edges`] settings, and samples each point with [`Sample::at_node`](crate::scene::Sample::at_node).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Graph {
  nodes: Vec<GraphNode>,
  /// For each node, its neighbors and the length of the edge to each, sorted by neighbor index
  neighbors: Vec<Vec<(usize, f32)>>,
}

impl Graph {
  /// Joins each pair of nodes that `edges` allows. Edge lengths are the distances between node positions.
  pub fn build(nodes: Vec<GraphNode>, edges: &Edges) -> Self {
    let mut neighbors = vec![Vec::new(); nodes.len()];
    let max_length = edges.max_length.unwrap_or(f32::INFINITY);

    for a in 0..nodes.len() {
      let pos = |i: usize| nodes[i].pos;
      let mut by_distance: Vec<(usize, f32)> = (0..nodes.len()).filter(|&b| b != a).map(|b| (b, pos(a).distance(pos(b)))).collect();
      by_distance.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));

      for (k, &(b, length)) in by_distance.iter().enumerate() {
        if length > max_length {
          break;
        }
        // each pair is considered from its lower index; the other side gets the edge then too
        if b < a {
          continue;
        }
        // a witness is always closer to `a` than `b` is, so only the points before `b` need checking
        let blocked = by_distance[..k].iter().any(|&(c, _)| edges.rule.is_witness(pos(a), pos(b), pos(c)));
        if !blocked {
          neighbors[a].push((b, length));
          neighbors[b].push((a, length));
        }
      }
    }

    neighbors.iter_mut().for_each(|list| list.sort_by_key(|(node, _)| *node));
    Self { nodes, neighbors }
  }

  pub fn nodes(&self) -> &[GraphNode] {
    &self.nodes
  }

  /// A node's neighbors with the length of the edge to each
  pub fn neighbors(&self, node: usize) -> &[(usize, f32)] {
    &self.neighbors[node]
  }

  /// Each edge once, as the indexes of its two nodes, lower first
  pub fn edges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
    self.neighbors.iter().enumerate().flat_map(|(a, list)| list.iter().filter(move |(b, _)| *b > a).map(move |(b, _)| (a, *b)))
  }

  /// The first node with this name
  pub fn find(&self, name: &str) -> Option<usize> {
    self.nodes.iter().position(|node| node.name == name)
  }

  /// The node closest to `pos`. `None` on an empty graph.
  pub fn nearest(&self, pos: Vec2) -> Option<usize> {
    (0..self.nodes.len()).min_by(|&a, &b| self.nodes[a].pos.distance_squared(pos).total_cmp(&self.nodes[b].pos.distance_squared(pos)))
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphNode {
  /// How traversals refer to it. On LEDs, the LED's name.
  pub name: String,
  pub pos: Vec2,
}

impl GraphNode {
  pub fn new(name: impl Into<String>, pos: Vec2) -> Self {
    Self { name: name.into(), pos }
  }
}

/// How a graph's nodes are joined, from their positions
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Edges {
  #[serde(default)]
  pub rule: EdgeRule,
  /// Longest edge allowed. Clusters further apart than this become separate pieces of the graph. `None` has no limit.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  #[cfg_attr(feature = "ts", ts(optional))]
  pub max_length: Option<f32>,
}

/// Which pairs of nodes get an edge. Both are subgraphs of the Delaunay triangulation, so edges don't cross.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum EdgeRule {
  /// Relative neighborhood graph: no other node is closer to both ends than they are to each other. Sparse and branching,
  /// like a tree.
  #[default]
  RelativeNeighborhood,
  /// Gabriel graph: no other node is inside the circle the edge is a diameter of. Denser, closer to a mesh. On an exact
  /// square grid, diagonals count as edges.
  Gabriel,
}

impl EdgeRule {
  /// Whether `c` rules out an edge from `a` to `b`
  fn is_witness(&self, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let length = a.distance(b);
    match self {
      Self::RelativeNeighborhood => a.distance(c) < length && b.distance(c) < length,
      Self::Gabriel => c.distance_squared((a + b) / 2.0) < (length / 2.0).powi(2),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// A `width` by `height` grid of nodes one apart, named by index
  fn grid(width: usize, height: usize, rule: EdgeRule) -> Graph {
    let nodes = (0..height).flat_map(|y| (0..width).map(move |x| Vec2::new(x as f32, y as f32)));
    let nodes = nodes.enumerate().map(|(i, pos)| GraphNode::new(i.to_string(), pos)).collect();
    Graph::build(nodes, &Edges { rule, max_length: None })
  }

  #[test]
  fn a_grid_joins_its_rows_and_columns() {
    let graph = grid(3, 3, EdgeRule::RelativeNeighborhood);
    assert_eq!(graph.edges().count(), 12);
    assert_eq!(graph.neighbors(4).iter().map(|(node, _)| *node).collect::<Vec<_>>(), [1, 3, 5, 7]);
  }

  #[test]
  fn gabriel_also_joins_the_diagonals_of_a_square() {
    assert_eq!(grid(2, 2, EdgeRule::Gabriel).edges().count(), 6);
    assert_eq!(grid(2, 2, EdgeRule::RelativeNeighborhood).edges().count(), 4);
  }

  #[test]
  fn a_point_between_two_others_splits_their_edge() {
    let nodes = [0.0, 1.0, 2.0].map(|x| GraphNode::new(format!("{x}"), Vec2::new(x, 0.0))).to_vec();
    let graph = Graph::build(nodes, &Edges::default());
    assert_eq!(graph.edges().collect::<Vec<_>>(), [(0, 1), (1, 2)]);
    assert_eq!(graph.neighbors(1), [(0, 1.0), (2, 1.0)]);
  }

  #[test]
  fn edges_longer_than_the_limit_are_left_out() {
    let nodes = [0.0, 1.0, 5.0].map(|x| GraphNode::new(format!("{x}"), Vec2::new(x, 0.0))).to_vec();
    let graph = Graph::build(nodes, &Edges { rule: EdgeRule::RelativeNeighborhood, max_length: Some(2.0) });
    assert_eq!(graph.edges().collect::<Vec<_>>(), [(0, 1)]);
    assert!(graph.neighbors(2).is_empty());
  }

  #[test]
  fn nodes_are_found_by_name_or_position() {
    let graph = grid(3, 1, EdgeRule::RelativeNeighborhood);
    assert_eq!(graph.find("2"), Some(2));
    assert_eq!(graph.find("nope"), None);
    assert_eq!(graph.nearest(Vec2::new(1.2, 3.0)), Some(1));
    assert_eq!(Graph::default().nearest(Vec2::ZERO), None);
  }
}
