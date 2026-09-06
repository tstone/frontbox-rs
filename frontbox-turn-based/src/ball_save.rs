use frontbox::provided::{AutoPlungerSystem, TroughFull, TroughSystem};
use frontbox::{prelude::*, provided::BallSaved};

use crate::ball_save::BallSaveState::*;
use crate::{GameManager, PlayerTurnActive, PlayerTurnEnding, TurnState};

/// Standard ball save at the start of a ball
#[derive(Clone)]
pub struct BallSaveSystem {
  duration: Duration,
  effect: Option<LedProgram1d>,
  cue: Option<u64>,
  state: BallSaveState,
}

impl BallSaveSystem {
  pub fn trough_interrupt_priority() -> u16 {
    25
  }

  pub fn new(initial_duration: Duration) -> Self {
    Self {
      duration: initial_duration,
      effect: None,
      cue: None,
      state: SaveNotActive,
    }
  }

  pub fn effect(mut self, effect: LedProgram1d) -> Self {
    self.effect = Some(effect.stopped());
    self
  }

  pub fn activate(&mut self, ctx: &SystemContext) {
    self.state = SaveActive;

    if let Some(effect) = self.effect.as_mut() {
      effect.play();
    }

    ctx.register_interrupt::<TroughFull>(Self::trough_interrupt_priority());
    ctx.cue(EndBallSave, Cue::Once(self.duration));
    log::info!("BallSave: Started");
  }

  pub fn deactivate(&mut self, ctx: &SystemContext) {
    self.state = SaveNotActive;
    ctx.unregister_interrupt::<TroughFull>();

    if let Some(cue_id) = self.cue {
      ctx.cancel_cue(cue_id);
    }

    if let Some(effect) = self.effect.as_mut() {
      effect.stop(ctx);
    }

    log::info!("BallSave: Ended");
  }
}

impl System for BallSaveSystem {
  fn is_active(&self, ctx: &SystemContext) -> bool {
    // Game is started and player state is Active || Ending
    if let Some(manager) = ctx.get::<GameManager>() {
      manager
        .game_state()
        .map(|game| {
          *game.current_player_turn_state() == TurnState::Active
            || *game.current_player_turn_state() == TurnState::Ending
        })
        .unwrap_or(false)
    } else {
      false
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<PlayerTurnActive>() {
      // automatically do ball save at the start of a turn
      self.activate(ctx);
    } else if event.is::<PlayerTurnEnding>() || event.is::<EndBallSave>() {
      // cancel any pending ball saves if the turn is over
      self.deactivate(ctx);
    }
  }

  fn on_interrupt(&mut self, _event: &dyn Event, ctx: &SystemContext) -> InterruptResult {
    // while active all TroughFull events are stopped
    log::info!("BallSave: interrupting TroughFull");
    ctx.emit(BallSaved);

    // Feed ball back to player
    if let Some(mut autoplunger) = ctx.get::<AutoPlungerSystem>() {
      autoplunger.eject_next();
    }
    if let Some(trough) = ctx.get::<TroughSystem>() {
      trough.eject(ctx.into());
    }

    InterruptResult::Halt
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    match (&self.state, self.effect.as_mut()) {
      (SaveActive, Some(effect)) => {
        effect.apply(delta, ctx);
      }
      _ => {}
    }
  }
}

// Cues
#[derive(serde::Serialize, Event)]
struct EndBallSave;

#[derive(Debug, Clone, PartialEq, Eq)]
enum BallSaveState {
  SaveActive,
  SaveNotActive,
}
