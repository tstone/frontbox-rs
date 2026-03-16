use std::vec;

use frontbox::prelude::*;

use crate::*;

#[derive(Debug, Clone, Serialize, Storable)]
pub struct PlayerScores {
  scores: Vec<u32>,
  multipliers: Vec<f32>,
}

impl PlayerScores {
  pub fn new(player_count: usize) -> Self {
    Self {
      scores: vec![0; player_count],
      multipliers: vec![1.0; player_count],
    }
  }
}

pub struct PlayerScoresSystem;

impl System for PlayerScoresSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<SetMultiplier>(move |command, _, ctx| {
      if let Some(game_state) = ctx.get::<PlayersGameState>() {
        let player_index = game_state.current_player() as usize;
        if let Some(scores) = ctx.get_mut::<PlayerScores>() {
          scores.multipliers[player_index] = command.0;
        }
      }
    });

    ctx.register_command::<AddPoints>(move |command, _, ctx| {
      if let Some(game_state) = ctx.get::<PlayersGameState>() {
        let player_index = game_state.current_player() as usize;

        let result = if let Some(scores) = ctx.get_mut::<PlayerScores>() {
          let multiplier = scores.multipliers[player_index];
          let points_received = (command.0 as f32 * multiplier) as u32;
          scores.scores[player_index] += points_received;
          let total_points = scores.scores[player_index];
          Some((points_received, total_points))
        } else {
          None
        };

        if let Some((points_received, total_points)) = result {
          ctx.emit(PointsAdded::new(
            player_index as u8,
            points_received,
            total_points,
          ));
        }
      }
    });
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(_) = event.downcast::<GameStarted>() {
      let player_count = ctx.expect::<PlayersGameState>().max_players as usize;
      ctx.insert(PlayerScores::new(player_count));
    } else if let Some(_) = event.downcast::<GameEnded>() {
      ctx.remove::<PlayerScores>();
    }
  }
}
