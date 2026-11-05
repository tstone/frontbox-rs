use crate::prelude::*;

pub struct CueTimeline {
  cues: Vec<(Duration, Box<dyn Event>)>,
}

impl CueTimeline {
  pub fn new() -> Self {
    Self { cues: vec![] }
  }

  pub fn cue_at(mut self, duration: Duration, signal: impl Event + 'static) -> Self {
    self.cues.push((duration, Box::new(signal)));
    self
  }

  pub(crate) fn points(self) -> Vec<(Duration, Box<dyn Event>)> {
    self.cues
  }
}
