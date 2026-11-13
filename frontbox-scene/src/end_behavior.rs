/// What to do when a scene is over
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EndBehavior {
  /// Remove last state
  Clear,
  /// Keep rendering final state
  Hold,
}
