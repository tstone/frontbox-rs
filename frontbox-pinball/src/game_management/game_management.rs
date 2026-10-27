use std::ops::{Deref, DerefMut};

use frontbox::prelude::*;

use crate::{
  CompetitiveGame, GameEnded, GameState, PlayerTurnBeginning, PlayerTurnEnding, TurnState,
};

pub trait GameManagement: SpawnableSystem {
  /// Adds a player to the game. Starts the game automatically if one has not yet been started.
  fn add_player(&mut self, ctx: &ServiceContext);
  /// Advances the turn to the next player. Typically should only be called once PlayerTurnEnd event has been fired.
  fn advance_turn(&mut self, ctx: &ServiceContext);
  /// Ends the game. Typically should only be called once GameEnding event has been fired.
  fn end_game(&mut self, ctx: &ServiceContext);

  fn is_player_addable(&self) -> bool;
  fn is_game_started(&self) -> bool;

  fn game_state(&self) -> Option<&GameState>;
  fn turn_state(&self) -> Option<&TurnState>;

  /// Add points to the current player/team
  fn add_points(&mut self, points: u32, ctx: &ServiceContext);
  /// Set points multiplier for current player/team
  fn set_multiplier(&mut self, multiplier: f32);
  /// Clear points multiplier for current player/team
  fn clear_multiplier(&mut self);
}

/// Runs the game through a `GameManagement` implementation, and makes the playfield live for each turn.
///
/// ## Playfield drivers
/// Drivers matching `playfield_drivers` that fire from a switch (slings, pops, flippers, etc.) are activated
/// automatically when a turn begins, and disabled when it ends or the game ends. Drivers fired by the game instead
/// (virtual switch triggers like ejects and resets) are left alone even when they match.
pub struct GameManager {
  inner: Box<dyn GameManagement>,
  playfield_drivers: Option<DriverQ>,
}

impl GameManager {
  pub fn new(game_management: impl GameManagement + 'static) -> Self {
    Self {
      inner: Box::new(game_management),
      playfield_drivers: None,
    }
  }

  pub fn competitive(
    max_players: u8,
    player_template: Vec<ChildSystemContainer>,
    ball_in_play_switches: SwitchQ,
  ) -> Self {
    Self::new(CompetitiveGame::new(
      max_players,
      player_template,
      ball_in_play_switches,
    ))
  }

  /// The drivers to make live during a turn (e.g. `DriverQ::tag::<Playfield>()`)
  pub fn playfield_drivers(mut self, drivers: DriverQ) -> Self {
    self.playfield_drivers = Some(drivers);
    self
  }

  fn switch_triggered_playfield_drivers(&self, ctx: &SystemContext) -> Vec<&'static str> {
    let (Some(query), Some(machine)) = (&self.playfield_drivers, ctx.get::<MachineSystem>()) else {
      return Vec::new();
    };
    query
      .query_names(ctx.into())
      .into_iter()
      .filter(|driver| machine.is_switch_triggered(driver, ctx.into()))
      .collect()
  }

  fn activate_playfield(&self, ctx: &SystemContext) {
    log::info!(target: "frontbox::game_manager", "Activating playfield drivers due to turn start");
    for driver in self.switch_triggered_playfield_drivers(ctx) {
      ctx.activate_driver(driver, ActivationMode::Automatic);
    }
  }

  fn deactivate_playfield(&self, ctx: &SystemContext) {
    log::info!(target: "frontbox::game_manager", "Deactivating playfield drivers due to turn end");
    for driver in self.switch_triggered_playfield_drivers(ctx) {
      ctx.deactivate_driver(driver, DeactivationMode::Disabled);
    }
  }
}

impl System for GameManager {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.inner.on_spawn(ctx)
  }

  fn on_deactivate(&mut self, ctx: &SystemContext) {
    self.inner.on_deactivate(ctx)
  }

  fn on_reactivate(&mut self, ctx: &SystemContext) {
    self.inner.on_reactivate(ctx)
  }

  fn on_despawn(&mut self, ctx: &SystemContext) {
    self.inner.on_despawn(ctx)
  }

  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    self.inner.on_tick(delta, ctx)
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<PlayerTurnBeginning>() {
      self.activate_playfield(ctx);
    } else if event.is::<PlayerTurnEnding>() || event.is::<GameEnded>() {
      self.deactivate_playfield(ctx);
    }
    self.inner.on_event(event, ctx)
  }

  fn on_interrupt(&mut self, event: &dyn Event, ctx: &SystemContext) -> InterruptResult {
    self.inner.on_interrupt(event, ctx)
  }

  fn is_active(&self, ctx: &SystemContext) -> bool {
    self.inner.is_active(ctx)
  }
}

impl Deref for GameManager {
  type Target = dyn GameManagement;

  fn deref(&self) -> &Self::Target {
    self.inner.as_ref()
  }
}

impl DerefMut for GameManager {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.as_mut()
  }
}
