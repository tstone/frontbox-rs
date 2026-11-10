use glam::Vec2;

/// Points joined by weighted edges, for a layer's [`GraphMask`](crate::scene::GraphMask) to run traversals over
///
/// Like sample positions, a graph comes from the device rather than the scene: the caller builds it from where its points
/// are, and samples each point with [`Sample::at_node`](crate::scene::Sample::at_node).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Graph {
  nodes: Vec<GraphNode>,
  /// For each node, its neighbors and the length of the edge to each, sorted by neighbor index
  neighbors: Vec<Vec<(usize, f32)>>,
}

impl Graph {
  /// Joins nodes into a Gabriel graph: two nodes get an edge when no other node is inside the circle the edge is a
  /// diameter of. That's a mesh with no crossing edges, apart from both diagonals of an exact square of nodes. Edge
  /// lengths are the distances between node positions.
  pub fn build(nodes: Vec<GraphNode>) -> Self {
    let mut neighbors = vec![Vec::new(); nodes.len()];

    for a in 0..nodes.len() {
      let pos = |i: usize| nodes[i].pos;
      let mut by_distance: Vec<(usize, f32)> = (0..nodes.len()).filter(|&b| b != a).map(|b| (b, pos(a).distance(pos(b)))).collect();
      by_distance.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));

      for (k, &(b, length)) in by_distance.iter().enumerate() {
        // each pair is considered from its lower index; the other side gets the edge then too
        if b < a {
          continue;
        }
        // a node inside the circle is always closer to `a` than `b` is, so only the points before `b` need checking
        let midpoint = (pos(a) + pos(b)) / 2.0;
        let blocked = by_distance[..k].iter().any(|&(c, _)| pos(c).distance_squared(midpoint) < (length / 2.0).powi(2));
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

#[cfg(test)]
mod tests {
  use super::*;

  /// A `width` by `height` grid of nodes one apart, named by index
  fn grid(width: usize, height: usize) -> Graph {
    let nodes = (0..height).flat_map(|y| (0..width).map(move |x| Vec2::new(x as f32, y as f32)));
    let nodes = nodes.enumerate().map(|(i, pos)| GraphNode::new(i.to_string(), pos)).collect();
    Graph::build(nodes)
  }

  #[test]
  fn a_grid_joins_its_rows_columns_and_diagonals() {
    let graph = grid(3, 3);
    assert_eq!(graph.edges().count(), 20);
    assert_eq!(graph.neighbors(4).iter().map(|(node, _)| *node).collect::<Vec<_>>(), [0, 1, 2, 3, 5, 6, 7, 8]);
  }

  #[test]
  fn a_point_between_two_others_splits_their_edge() {
    let nodes = [0.0, 1.0, 2.0].map(|x| GraphNode::new(format!("{x}"), Vec2::new(x, 0.0))).to_vec();
    let graph = Graph::build(nodes);
    assert_eq!(graph.edges().collect::<Vec<_>>(), [(0, 1), (1, 2)]);
    assert_eq!(graph.neighbors(1), [(0, 1.0), (2, 1.0)]);
  }

  #[test]
  fn nodes_are_found_by_name_or_position() {
    let graph = grid(3, 1);
    assert_eq!(graph.find("2"), Some(2));
    assert_eq!(graph.find("nope"), None);
    assert_eq!(graph.nearest(Vec2::new(1.2, 3.0)), Some(1));
    assert_eq!(Graph::default().nearest(Vec2::ZERO), None);
  }
}
