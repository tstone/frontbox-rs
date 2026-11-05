use crate::prelude::*;

pub trait CueExt {
  fn once(&self) -> Cue;
  fn times(&self, n: u16) -> Cue;
  fn forever(&self) -> Cue;
}

impl CueExt for Duration {
  fn once(&self) -> Cue {
    Cue::Once(*self)
  }

  fn times(&self, n: u16) -> Cue {
    Cue::Times(n, *self)
  }

  fn forever(&self) -> Cue {
    Cue::Forever(*self)
  }
}
