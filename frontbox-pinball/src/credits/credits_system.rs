use std::path::{Path, PathBuf};

use frontbox_core::prelude::*;
use frontbox_core::toml;
use frontbox_lighting::prelude::*;

use crate::credits::{CreditsChanged, StartRefused, StartRefusedReason};
use crate::{GameManagerExt, GamePhase, PlayerEntry, configs, metrics};

/// Counts coins into credits, and adds a player when the start button is pressed while one can be added
/// (`PlayerEntry::Open`), spending a credit for them. With `configs::FREE_PLAY` on, a player is added without one.
///
/// Every `configs::COINS_PER_CREDIT` coins, from any of the coin switches, make a credit, up to `configs::MAX_CREDITS`.
/// Coins count at any time, during a game and on free play too, so money put in isn't lost. A press that adds no
/// player (no credit with free play off, or a game that can't take one) emits `StartRefused` with the reason, for the
/// game to give feedback.
///
/// With a start lamp, it declares the lamp flashing through light management while a press would add a player, and
/// takes the declaration back otherwise.
///
/// Credits are kept across power cycles when given a file to save them to (`save_to`); without one, they're kept in
/// memory and start at 0.
///
/// ## Setup
/// ```rust,ignore
/// // at the root
/// CreditsSystem::new(start_button::SWITCH.q())
///   .coin_switches(SwitchQ::tag::<CoinSwitch>())
///   .start_lamp(start_button::LAMP.q())
///   .save_to("credits.toml")
/// ```
pub struct CreditsSystem {
  start_button: SwitchQ,
  coin_switches: Option<SwitchQ>,
  start_lamp: Option<LightQ>,
  /// Whether the start lamp is declared flashing
  lamp_lit: bool,
  saved: SavedCredits,
  path: Option<PathBuf>,
  handle: SystemHandle,
}

impl CreditsSystem {
  pub fn new(start_button: impl Into<SwitchQ>) -> Self {
    Self {
      start_button: start_button.into(),
      coin_switches: None,
      start_lamp: None,
      lamp_lit: false,
      saved: SavedCredits::default(),
      path: None,
      handle: SystemHandle::default(),
    }
  }

  /// The switches coins drop through. Each close is one coin.
  pub fn coin_switches(mut self, switches: impl Into<SwitchQ>) -> Self {
    self.coin_switches = Some(switches.into());
    self
  }

  /// The start button's lamp, such as a driver light (`DriverDefinition::light`) or an LED
  pub fn start_lamp(mut self, lamp: impl Into<LightQ>) -> Self {
    self.start_lamp = Some(lamp.into());
    self
  }

  /// The file credits are saved to and loaded from, so they're kept across power cycles. Loads it now.
  pub fn save_to(mut self, path: impl AsRef<Path>) -> Self {
    let path = path.as_ref();
    self.saved = toml::load(path).unwrap_or_default();
    self.path = Some(path.to_path_buf());
    self
  }

  pub fn credits(&self) -> u8 {
    self.saved.credits
  }

  /// Adds `count` credits, up to `configs::MAX_CREDITS`, e.g. service credits from the operator menu
  pub fn add_credits(&mut self, count: u8, ctx: &ServiceContext) {
    let ctx = &ctx.for_system(self.handle);
    let max = configs::MAX_CREDITS.get(ctx);
    if self.saved.credits >= max {
      log::info!(target: "frontbox::credits", "Adding {count} credits at the credit limit, none added");
      return;
    }
    let credits = self.saved.credits.saturating_add(count).min(max);
    log::info!(target: "frontbox::credits", "Adding {count} credits: {} => {credits}", self.saved.credits);
    self.saved.credits = credits;
    self.changed(ctx);
  }

  fn on_coin(&mut self, ctx: &SystemContext) {
    ctx.metrics.inc(&metrics::COINS_INSERTED);
    if self.saved.credits >= configs::MAX_CREDITS.get(ctx) {
      log::info!(target: "frontbox::credits", "Coin inserted at the credit limit, no credit added");
      return;
    }

    self.saved.coins += 1;
    if self.saved.coins >= configs::COINS_PER_CREDIT.get(ctx) {
      self.saved.coins = 0;
      self.saved.credits += 1;
    }
    log::info!(
      target: "frontbox::credits",
      "Coin inserted: {} credits, {} coins toward the next",
      self.saved.credits,
      self.saved.coins
    );
    self.changed(ctx);
  }

  fn on_start_pressed(&mut self, ctx: &SystemContext) {
    if !ctx.states.is(PlayerEntry::Open) {
      Self::refuse(Self::entry_closed_reason(ctx), ctx);
      return;
    }

    let free_play = configs::FREE_PLAY.get(ctx);
    if !free_play && self.saved.credits == 0 {
      Self::refuse(StartRefusedReason::NoCredits, ctx);
      return;
    }

    let Some(mut game_manager) = ctx.game_manager() else {
      log::warn!(target: "frontbox::credits", "No game manager running, can't add a player");
      return;
    };
    let added = game_manager.add_player(ctx.into());
    drop(game_manager);
    if !added {
      Self::refuse(Self::entry_closed_reason(ctx), ctx);
      return;
    }
    if free_play {
      return;
    }

    self.saved.credits -= 1;
    ctx.metrics.inc(&metrics::CREDITS_SPENT);
    log::info!(target: "frontbox::credits", "Credit spent adding a player, {} left", self.saved.credits);
    self.changed(ctx);
  }

  fn refuse(reason: StartRefusedReason, ctx: &SystemContext) {
    log::info!(target: "frontbox::credits", "Start pressed, no player added: {reason:?}");
    ctx.emit(StartRefused { reason });
  }

  /// Why the game manager can't take a player, from the game's phase
  fn entry_closed_reason(ctx: &SystemContext) -> StartRefusedReason {
    match ctx.states.get::<GamePhase>() {
      GamePhase::None => StartRefusedReason::BallsReturning,
      GamePhase::Starting | GamePhase::Active => StartRefusedReason::GameInProgress,
      GamePhase::Ending => StartRefusedReason::GameEnding,
    }
  }

  /// Whether a start press now would add a player
  fn is_startable(&self, ctx: &SystemContext) -> bool {
    ctx.states.is(PlayerEntry::Open) && (self.saved.credits > 0 || configs::FREE_PLAY.get(ctx))
  }

  fn changed(&self, ctx: &SystemContext) {
    ctx.emit(CreditsChanged {
      credits: self.saved.credits,
      coins: self.saved.coins,
    });
    if let Some(path) = &self.path
      && let Err(e) = toml::save(path, &self.saved)
    {
      log::error!(target: "frontbox::credits", "Failed to save credits to {}: {e}", path.display());
    }
  }

  /// Declares the start lamp flashing while a press would add a player, and takes it back once one wouldn't
  fn update_lamp(&mut self, ctx: &SystemContext) {
    let lit = self.start_lamp.is_some() && self.is_startable(ctx);
    if lit == self.lamp_lit {
      return;
    }
    self.lamp_lit = lit;
    let Some(lamp) = &self.start_lamp else {
      return;
    };
    if lit {
      ctx.declare_lights_flashing(lamp, LAMP_Z, Rgba::white());
    } else {
      ctx.undeclare_lights(lamp, LAMP_Z);
    }
  }
}

impl System for CreditsSystem {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();
  }

  fn on_tick(&mut self, _delta: Duration, ctx: &SystemContext) {
    self.update_lamp(ctx);
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>() {
      if self.start_button.matches(switch) {
        self.on_start_pressed(ctx);
      } else if let Some(coins) = &self.coin_switches
        && coins.matches(switch)
      {
        self.on_coin(ctx);
      }
    }
  }
}

/// What's kept across power cycles
#[derive(Debug, Default, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct SavedCredits {
  credits: u8,
  /// Coins toward the next credit
  coins: u8,
}

/// The z the start lamp is declared at
const LAMP_Z: i8 = 0;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{GameEndReason, GameManager, GameSession};

  const START: &str = "start";

  /// A game manager that refuses every player, as it does when `PlayerEntry` hasn't caught up with a game that can no
  /// longer take one
  struct RefusingGame;

  impl System for RefusingGame {
    fn provides(provides: &mut Provides<Self>) {
      provides.add::<dyn GameManager>(|s| s);
    }
  }

  impl GameManager for RefusingGame {
    fn add_player(&mut self, _ctx: &ServiceContext) -> bool {
      false
    }
    fn quit_game(&mut self, _reason: GameEndReason, _ctx: &ServiceContext) {}
    fn tilt(&mut self, _ctx: &ServiceContext) {}
    fn add_player_system(&mut self, _system: ChildSystemContainer) {}
    fn player_group(&self, _player: u8) -> Option<u64> {
      None
    }
    fn session(&self) -> Option<&GameSession> {
      None
    }
    fn add_points(&mut self, _points: u64, _ctx: &ServiceContext) {}
    fn set_multiplier(&mut self, _multiplier: f32) {}
    fn clear_multiplier(&mut self) {}
    fn grant_extra_ball(&mut self, _ctx: &ServiceContext) -> bool {
      false
    }
  }

  /// Stands in for the game manager, which owns `PlayerEntry`
  struct EntryOwner;
  impl System for EntryOwner {}

  #[test]
  fn a_player_the_game_refuses_spends_no_credit() {
    let mut ctx = TestContext::default();
    ctx.svc_ctx().config.set(&configs::FREE_PLAY, false);
    ctx.insert_system(RefusingGame);
    let owner = ctx.insert_system(EntryOwner);
    ctx.svc_ctx().for_system(owner).states.set(PlayerEntry::Open);
    ctx.apply_states();

    let mut credits = CreditsSystem::new(SwitchQ::name(START));
    credits.on_spawn(&ctx.sys_ctx());
    credits.add_credits(1, &ctx.svc_ctx());

    credits.on_event(
      &SwitchClosed::new(Switch {
        name: START,
        ..Default::default()
      }),
      &ctx.sys_ctx(),
    );
    assert_eq!(credits.credits(), 1);
  }
}
