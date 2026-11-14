use std::fmt;

/// Why a scene can't be played
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
  /// A layer has props that change over time, but there's no timing to play them in
  NoTiming { layer: String },
  /// The timing's duration isn't a positive number
  NoDuration,
  /// Two traversals in a layer's graph have the same name
  SameTraversalName { layer: String, traversal: String },
  /// A traversal's fixed rate isn't a positive number
  NoSpeed { layer: String, traversal: String },
  /// A sound starts before the scene does, or at or after the end of a cycle
  StartOutsideCycle { layer: String },
  /// A sound is cut off at or before its start
  EndBeforeStart { layer: String },
  /// A sound skips less than none of its beginning
  NegativeSkip { layer: String },
}

impl fmt::Display for SceneError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoTiming { layer } => write!(f, "{layer} changes over time, so the scene needs a timing"),
      Self::NoDuration => write!(f, "a scene's duration must be more than zero"),
      Self::SameTraversalName { layer, traversal } => write!(f, "{layer} has more than one traversal named \"{traversal}\""),
      Self::NoSpeed { layer, traversal } => write!(f, "traversal \"{traversal}\" in {layer} needs a rate or duration more than zero"),
      Self::StartOutsideCycle { layer } => write!(f, "{layer} has to start within a cycle of the scene"),
      Self::EndBeforeStart { layer } => write!(f, "{layer} has to end after it starts"),
      Self::NegativeSkip { layer } => write!(f, "{layer} can't skip less than none of its sound"),
    }
  }
}

impl std::error::Error for SceneError {}
