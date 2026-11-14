use glam::Vec2;

/// Every visit a layer's traversals make in the current cycle. Get it from [`Layer::visits`](crate::scene::Layer::visits)
/// and pass it in [`LayerFrame::visits`](crate::scene::LayerFrame::visits).
#[derive(Debug, Clone, Default)]
pub struct Visits {
  /// In the order of the layer's traversals
  pub(crate) traversals: Vec<TraversalVisits>,
}

impl Visits {
  /// The visits the layer's responses see at one node
  pub(crate) fn at(&self, node: usize) -> NodeVisits<'_> {
    NodeVisits { traversals: &self.traversals, node }
  }

  /// Seconds from the first visit to the last for each of the layer's traversals, in its order. `None` for one that
  /// visits nothing.
  pub fn lengths(&self) -> Vec<Option<f32>> {
    self.traversals.iter().map(TraversalVisits::length).collect()
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

impl TraversalVisits {
  fn length(&self) -> Option<f32> {
    let first = self.by_node.iter().filter_map(|times| times.first()).copied().reduce(f32::min)?;
    let last = self.by_node.iter().filter_map(|times| times.last()).copied().fold(first, f32::max);
    Some(last - first)
  }
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
