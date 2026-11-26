use std::fmt;

/// Why a scene can't be played
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
  /// A layer has props that change over time, but there's no timing to play them in
  NoTiming { layer: String },
  /// The timing's duration isn't a positive number
  NoDuration,
  /// A modulator's wave goes by less than once a cycle
  ModulatorTooSlow { modulator: String },
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
  /// An external envelope follows a sound layer the scene doesn't have
  NoSoundToFollow { sound: String },
  /// An external envelope's attack or release isn't 0 or more seconds
  NegativeSlew { sound: String },
  /// Part of a layer goes behind a layer that isn't under it on the same thing at the same z
  NoLayerBehind { layer: String, behind: String },
  /// Part of a layer goes down to a z that isn't lower than the layer's own
  NotBehindZ { layer: String, z: i8 },
  /// A sweep follows a layer the scene doesn't have
  NoPathToFollow { layer: String, path: String },
  /// A sweep follows a layer by a name more than one drawn layer has
  SamePathName { path: String },
  /// A sweep follows a layer with no region
  PathHasNoRegion { path: String },
  /// A sweep follows a layer drawn on something else, whose units aren't its own
  PathElsewhere { layer: String, path: String },
  /// A layer's region follows a region that follows its way back to the first
  PathLoop { layer: String },
}

impl fmt::Display for SceneError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoTiming { layer } => {
        write!(f, "{layer} changes over time, so the scene needs a timing")
      }
      Self::NoDuration => write!(f, "a scene's duration must be more than zero"),
      Self::ModulatorTooSlow { modulator } => write!(
        f,
        "modulator \"{modulator}\" needs a rate of at least 1 wave per cycle"
      ),
      Self::SameTraversalName { layer, traversal } => write!(
        f,
        "{layer} has more than one traversal named \"{traversal}\""
      ),
      Self::NoSpeed { layer, traversal } => write!(
        f,
        "traversal \"{traversal}\" in {layer} needs a rate or duration more than zero"
      ),
      Self::StartOutsideCycle { layer } => {
        write!(f, "{layer} has to start within a cycle of the scene")
      }
      Self::EndBeforeStart { layer } => write!(f, "{layer} has to end after it starts"),
      Self::NegativeSkip { layer } => write!(f, "{layer} can't skip less than none of its sound"),
      Self::NoSoundToFollow { sound } => write!(
        f,
        "an envelope follows sound layer \"{sound}\", which the scene doesn't have"
      ),
      Self::NegativeSlew { sound } => write!(
        f,
        "the envelope following \"{sound}\" needs an attack and release of 0 or more seconds"
      ),
      Self::NoLayerBehind { layer, behind } => write!(
        f,
        "{layer} goes behind layer \"{behind}\", which isn't under it on the same thing at the same z"
      ),
      Self::NotBehindZ { layer, z } => write!(
        f,
        "{layer} has a part that goes behind at z {z}, which has to be lower than its own z"
      ),
      Self::NoPathToFollow { layer, path } => write!(
        f,
        "{layer} sweeps along layer \"{path}\", which the scene doesn't have"
      ),
      Self::SamePathName { path } => write!(
        f,
        "a sweep follows layer \"{path}\", but more than one layer has that name"
      ),
      Self::PathHasNoRegion { path } => write!(
        f,
        "a sweep follows layer \"{path}\", which has no region to follow"
      ),
      Self::PathElsewhere { layer, path } => write!(
        f,
        "{layer} sweeps along layer \"{path}\", which is drawn on something else"
      ),
      Self::PathLoop { layer } => write!(
        f,
        "the region of {layer} sweeps along regions that come back around to it"
      ),
    }
  }
}

impl std::error::Error for SceneError {}
