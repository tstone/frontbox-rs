use frontbox::prelude::*;

use crate::ball_management::PerformBallSearch;
use crate::configs;

/// The way this will work is a hybrid of tagging + event handling. Searches after `BALL_SEARCH_DELAY` (an operator
/// config) without a ball present switch closing.
pub struct BallSearchSystem {
  cue_id: Option<u64>,
  handle: SystemHandle,
  ball_present_q: SwitchQ,
  ball_searchable_q: DriverQ,
}

impl BallSearchSystem {
  pub fn new(ball_present_q: SwitchQ, ball_searchable_q: DriverQ) -> Self {
    Self {
      cue_id: None,
      handle: SystemHandle::default(),
      ball_present_q,
      ball_searchable_q,
    }
  }

  fn reset(&mut self, ctx: &SystemContext) {
    self.clear_cue(&ctx);
    let delay = ctx.config.get(&configs::BALL_SEARCH_DELAY);
    self.cue_id = Some(ctx.cue(StartBallSearch, delay.once()));
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
