use glam::Vec2;

/// Every visit each layer's traversals make in the current cycle. Get it from
/// [`GfxComposition::visits`](crate::scene::GfxComposition::visits) and pass it in
/// [`FrameContext::visits`](crate::scene::FrameContext::visits).
#[derive(Debug, Clone, Default)]
pub struct Visits {
  /// By layer, then by traversal in the layer's order. Empty for a layer without a graph.
  pub(crate) layers: Vec<Vec<TraversalVisits>>,
}

impl Visits {
  /// The visits a layer's responses see at one node
  pub(crate) fn at(&self, layer: usize, node: usize) -> Option<NodeVisits<'_>> {
    self.layers.get(layer).map(|traversals| NodeVisits { traversals, node })
  }
}

#[derive(Debug, Clone)]
pub(crate) struct TraversalVisits {
  pub name: String,
  /// Steps per second, to convert a response's timing to seconds
  pub rate: f32,
  /// Seconds into the cycle of each visit, sorted, by node index
  pub by_node: Vec<Vec<f32>>,
  /// Positions of the start node and the node visited last, for [`Transform::Fit`](crate::scene::Transform::Fit).
  /// `None` if nothing is visited.
  pub ends: Option<(Vec2, Vec2)>,
}

/// One layer's visits at one node
#[derive(Debug, Clone, Copy)]
pub(crate) struct NodeVisits<'a> {
  traversals: &'a [TraversalVisits],
  node: usize,
}

impl<'a> NodeVisits<'a> {
  /// This node's visit times from the named traversal, and its rate. `None` if there's no traversal by that name.
  pub fn of(&self, traversal: &str) -> Option<(&'a [f32], f32)> {
    let found = self.find(traversal)?;
    Some((found.by_node.get(self.node).map_or(&[][..], Vec::as_slice), found.rate))
  }

  /// Where the named traversal starts and ends this cycle
  pub fn ends(&self, traversal: &str) -> Option<(Vec2, Vec2)> {
    self.find(traversal)?.ends
  }

  fn find(&self, traversal: &str) -> Option<&'a TraversalVisits> {
    self.traversals.iter().find(|visits| visits.name == traversal)
  }
}
