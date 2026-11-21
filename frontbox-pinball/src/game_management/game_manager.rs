use frontbox_core::prelude::*;

use crate::{GameEndReason, GameSession};

/// The game manager runs the game: players, turns, points and extra balls. `CompetitiveGame` is the standard one.
///
/// Other systems find it by this trait, `ctx.systems.get_dyn::<dyn GameManager>()` or `ctx.game_manager()`, so an
/// implementation lists it in [`System::provides`]. Making the playfield live for each turn is `PlayfieldDrivers`'s
/// job, not the game manager's.
pub trait GameManager: SpawnableSystem {
  /// Adds a player to the game. Starts the game automatically if one has not yet been started. Does nothing if no
  /// player can be added right now; `PlayerEntry` says whether one can. Returns whether a player was added.
  fn add_player(&mut self, ctx: &ServiceContext) -> bool;
  /// Ends the game in progress early, skipping the rest of the current turn. `GameEnding` still runs, with `reason`
  /// (`GameEndReason::Quit`, or `SlamTilted`). Does nothing if no game is started, or it's already ending.
  fn quit_game(&mut self, reason: GameEndReason, ctx: &ServiceContext);
  /// The current player tilts: their turn ends at once (`TurnEndReason::Tilted`), points are ignored for the rest of
  /// it, and drains can't be saved. Does nothing unless the turn's ball has been served, or once the turn is ending.
  fn tilt(&mut self, ctx: &ServiceContext);

  /// Adds a system to the player template, so every player added from now on gets their own copy. Used at setup for
  /// the per-player children an implementation is configured with (e.g. `CompetitiveGame::tilt_bob`).
  fn add_player_system(&mut self, system: ChildSystemContainer);

  fn session(&self) -> Option<&GameSession>;

  /// The id of the player's `Group`, which holds their copy of the player template. A system outside it looks up one
  /// of the player's systems with `ctx.systems.get_in::<T>(group)`. `None` with no game started, or no such player.
  fn player_group(&self, player: u8) -> Option<u64>;

  /// Add points to the current player/team
  fn add_points(&mut self, points: u64, ctx: &ServiceContext);
  /// Set points multiplier for current player/team
  fn set_multiplier(&mut self, multiplier: f32);
  /// Clear points multiplier for current player/team
  fn clear_multiplier(&mut self);

  /// Give the current player an extra ball to be used after the current turn ends
  fn grant_extra_ball(&mut self, ctx: &ServiceContext) -> bool;
}
