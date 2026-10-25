use std::time::Duration;

use frontbox::prelude::*;

use crate::ball_management::PerformBallSearch;

/// The way this will work is a hybrid of tagging + event handling
///
pub struct BallSearchSystem {
  cue_id: Option<u64>,
  handle: SystemHandle,
  time_until_search: Duration,
  ball_present_q: SwitchQ,
  ball_searchable_q: DriverQ,
}

impl BallSearchSystem {
  pub fn new(
    ball_present_q: SwitchQ,
    ball_searchable_q: DriverQ,
    time_until_search: Duration,
  ) -> Self {
    Self {
      cue_id: None,
      handle: SystemHandle::default(),
      time_until_search,
      ball_present_q,
      ball_searchable_q,
    }
  }

  fn reset(&mut self, ctx: &SystemContext) {
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
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>()
      && self.ball_present_q.matches(&event.switch)
    {
      self.reset(ctx.into());
    } else if event.is::<StartBallSearch>() {
      log::info!(target: "frontbox::ball_search", "Starting ball search.");
      ctx.emit(PerformBallSearch);
    }
  }
}

// -- Cues --

#[derive(serde::Serialize, Event)]
struct StartBallSearch;
