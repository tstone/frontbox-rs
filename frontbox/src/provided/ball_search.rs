use std::time::Duration;

use crate::prelude::*;


pub struct BallSearchSystem {
  cue_id: Option<u64>,
  handle: SystemHandle,
  time_until_search: Duration,
}

impl BallSearchSystem {
  pub fn new(time_until_search: Duration) -> Self {
    Self {
      cue_id: None,
      handle: SystemHandle::default(),
      time_until_search,
    }
  }

  pub fn reset(&mut self, ctx: &ServiceContext) {
    let ctx = ctx.for_system(self.handle);
    self.clear_cue(&ctx);
    self.cue_id = Some(ctx.cue(StartBallSearch, self.time_until_search.once()));
  }

  fn clear_cue(&mut self, ctx: &SystemContext) {
    if let Some(cue_id) = self.cue_id.as_ref() {
      ctx.cancel_cue(*cue_id);
      self.cue_id = None;
    }
  }
}

impl System for BallSearchSystem {
  fn on_event(&mut self, event: &dyn crate::prelude::Event, ctx: &crate::prelude::SystemContext) {
    if event.is::<SwitchClosed>() {
      self.reset(ctx.into());
    } else if event.is::<StartBallSearch>() {
      ctx.emit(BallSearch);
    }
  }
}

#[derive(serde::Serialize, Event)]
pub struct StartBallSearch;

#[derive(serde::Serialize, Event)]
pub struct BallSearch;