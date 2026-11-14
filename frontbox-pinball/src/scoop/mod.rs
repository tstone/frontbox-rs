mod scoop_system;

pub use scoop_system::*;

use std::marker::PhantomData;

use frontbox_core::prelude::*;

// -- Scoop Events --
// Each is generic over the scoop's marker type, as `ScoopSystem` is, so `ScoopEjected<Lower>` is only ever the lower
// scoop's.

/// A ball came to rest in the scoop. Anything that wants it to wait there calls `ScoopSystem::hold` on hearing this;
/// otherwise the eject starts.
#[derive(serde::Serialize, Event)]
pub struct ScoopBallEntered<S> {
  pub entry: ScoopEntry,
  #[serde(skip)]
  scoop: PhantomData<fn() -> S>,
}

impl<S> ScoopBallEntered<S> {
  pub fn new(entry: ScoopEntry) -> Self {
    Self {
      entry,
      scoop: PhantomData,
    }
  }
}

/// How a ball got into a scoop
#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoopEntry {
  /// Through the scoop's opening on the playfield
  Direct,
  /// Through an entrance below the playfield (e.g. a subway), having passed `ScoopSystem::lower_entrance_switch` on
  /// the way
  LowerEntrance,
}

/// The scoop is about to eject its ball, once its eject scene has played (right away without one). A hold placed
/// before then stops the eject, and this is emitted again when it next starts.
#[derive(serde::Serialize, Event)]
pub struct ScoopEjectStarted<S> {
  #[serde(skip)]
  scoop: PhantomData<fn() -> S>,
}

impl<S> ScoopEjectStarted<S> {
  pub fn new() -> Self {
    Self { scoop: PhantomData }
  }
}

/// The ball has been kicked out of the scoop. Emitted once per ball, not for a kick that's repeated because the ball
/// didn't leave.
#[derive(serde::Serialize, Event)]
pub struct ScoopEjected<S> {
  #[serde(skip)]
  scoop: PhantomData<fn() -> S>,
}

impl<S> ScoopEjected<S> {
  pub fn new() -> Self {
    Self { scoop: PhantomData }
  }
}
