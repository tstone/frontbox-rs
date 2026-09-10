mod multiball_ext;
pub use multiball_ext::*;

use frontbox_derive::Event;

use crate::prelude::*;
use crate::provided::multiball::State::*;
use crate::provided::{
  AutoPlungerSystem, BallEnteredTrough, BallExitedPlungeLane, TroughFull, TroughSystem,
};

#[derive(Clone)]
pub struct MultiballSystem {
  handle: SystemHandle,
  effect: LedProgram1d,
  ball_save_time: Duration,
  state: State,
}

impl MultiballSystem {
  pub fn new(ball_save_time: Duration, ball_save_effect: LedProgram1d) -> Self {
    Self {
      handle: SystemHandle::default(),
      effect: ball_save_effect.stopped(),
      ball_save_time,
      state: MultiballInactive,
    }
  }

  pub fn is_multiball_active(&self) -> bool {
    self.state != MultiballInactive
  }

  pub fn add_balls(&mut self, balls: u8, ctx: &ServiceContext) {
    if balls == 0 {
      log::warn!(target: "frontbox::multiball", "Cannot start multiball with 0 additional balls.");
      return;
    }

    let ctx = &ctx.for_system(self.handle);
    match self.state {
      MultiballInactive => {
        log::info!(target: "frontbox::multiball", "Starting multiball with additional balls: {}", balls);
        ctx.register_interrupt::<TroughFull>(25);
        ctx.register_interrupt::<BallEnteredTrough>(25);
        self.start_launching_balls(balls, balls, ctx);
      }
      LaunchingAdditionalBalls {
        addl_ball_count,
        remaining_balls_to_launch: remaining_balls_to_eject,
      } => {
        log::info!(target: "frontbox::multiball", "Adding additional multiball balls (launching): {}", balls);
        self.state = LaunchingAdditionalBalls {
          addl_ball_count: addl_ball_count + balls,
          remaining_balls_to_launch: remaining_balls_to_eject + balls,
        };
      }
      BallSaveActive {
        addl_ball_count,
        cue_id,
      } => {
        log::info!(target: "frontbox::multiball", "Adding additional multiball balls (ball save): {}", balls);
        ctx.cancel_cue(cue_id);
        self.start_launching_balls(addl_ball_count + balls, balls, ctx);
      }
      MultiballActive { addl_ball_count } => {
        log::info!(target: "frontbox::multiball", "Adding additional multiball balls (active): {}", balls);
        ctx.register_interrupt::<TroughFull>(25);
        ctx.register_interrupt::<BallEnteredTrough>(25);
        self.start_launching_balls(addl_ball_count + balls, balls, ctx);
      }
    }
  }

  fn start_launching_balls(&mut self, addl_ball_count: u8, to_launch: u8, ctx: &SystemContext) {
    self.effect.play();
    self.launch_ball(ctx);

    let remaining_balls_to_launch = to_launch - 1;
    if remaining_balls_to_launch > 0 {
      self.state = LaunchingAdditionalBalls {
        addl_ball_count,
        remaining_balls_to_launch,
      }
    } else {
      self.start_ball_save(addl_ball_count, ctx);
    }
  }

  fn launch_ball(&self, ctx: &SystemContext) {
    log::debug!(target: "frontbox::multiball", "Launching additional ball for multiball");
    if let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() {
      autoplunger.eject_next();
    }
    ctx.expect::<TroughSystem>().eject(ctx.into());
  }

  fn start_ball_save(&mut self, addl_ball_count: u8, ctx: &SystemContext) {
    log::debug!(target: "frontbox::multiball", "Starting multiball ball save");
    self.state = BallSaveActive {
      addl_ball_count,
      cue_id: ctx.cue(EndBallSave, self.ball_save_time.once()),
    };
  }

  fn end_ball_save(&mut self, addl_ball_count: u8, ctx: &SystemContext) {
    log::debug!(target: "frontbox::multiball", "Ending multiball ball save");
    self.effect.stop(ctx);
    self.state = MultiballActive { addl_ball_count };
    ctx.unregister_interrupt::<TroughFull>();
    ctx.unregister_interrupt::<BallEnteredTrough>();
  }
}

impl System for MultiballSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = *ctx.current_handle();
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<EndBallSave>()
      && let BallSaveActive {
        addl_ball_count, ..
      } = self.state
    {
      self.end_ball_save(addl_ball_count, ctx);
    } else if event.is::<BallExitedPlungeLane>()
      && let LaunchingAdditionalBalls {
        addl_ball_count,
        remaining_balls_to_launch,
      } = self.state
    {
      self.start_launching_balls(addl_ball_count, remaining_balls_to_launch, ctx);
    } else if event.is::<BallEnteredTrough>()
      && let MultiballActive { addl_ball_count } = self.state
    {
      let rem = addl_ball_count - 1;
      log::debug!(target: "frontbox::multiball", "Additional multiball drained. Remaining: {}", rem);

      if rem == 0 {
        log::info!(target: "frontbox::multiball", "Multiball: All additional balls drained. Ending multiball.");
        ctx.emit(MultiballEnded);
        self.state = MultiballInactive;
      } else {
        self.state = MultiballActive {
          addl_ball_count: rem,
        }
      }
    }
  }

  fn on_interrupt(&mut self, event: &dyn Event, ctx: &SystemContext) -> InterruptResult {
    if event.is::<TroughFull>() {
      return InterruptResult::Halt;
    } else if event.is::<BallEnteredTrough>() {
      log::debug!(target: "frontbox::multiball", "Multiball: Re-launching ball drained during multiball");
      self.launch_ball(ctx);
    }
    InterruptResult::Continue
  }

  fn on_tick(&mut self, delta: std::time::Duration, ctx: &SystemContext) {
    self.effect.apply(delta, ctx);
  }
}

#[derive(serde::Serialize, Event)]
struct EndBallSave;

#[derive(Debug, Clone, PartialEq, Eq)]
enum State {
  MultiballInactive,
  LaunchingAdditionalBalls {
    addl_ball_count: u8,
    remaining_balls_to_launch: u8,
  },
  BallSaveActive {
    addl_ball_count: u8,
    cue_id: u64,
  },
  MultiballActive {
    addl_ball_count: u8,
  },
}

#[derive(serde::Serialize, Event)]
pub struct MultiballEnded;
