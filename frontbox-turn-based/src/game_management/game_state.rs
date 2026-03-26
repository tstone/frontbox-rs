use std::collections::HashMap;

use crate::TurnState;

pub enum GameState {
  Competitive {
    player_count: u8,
    max_players: u8,
    current_player: u8,
    current_turn_state: TurnState,
    player_turns: Vec<u8>,
    player_multiplier: Vec<f32>,
    player_scores: Vec<u32>,
  },
  Cooperative {
    player_count: u8,
    team_count: u8,
    player_team_map: HashMap<u8, u8>,
    max_players: u8,
    current_player: u8,
    current_team: u8,
    current_turn_state: TurnState,
    player_turns: Vec<u8>,
    player_multiplier: Vec<f32>,
    player_scores: Vec<u32>,
  },
}

impl GameState {
  pub fn new(max_players: u8) -> Self {
    GameState::Competitive {
      player_count: 0,
      max_players,
      current_player: 0,
      current_turn_state: TurnState::Beginning,
      player_turns: vec![0; max_players as usize],
      player_multiplier: vec![1.0; max_players as usize],
      player_scores: vec![0; max_players as usize],
    }
  }

  pub fn current_player_turn_state(&self) -> &TurnState {
    match self {
      GameState::Competitive {
        current_turn_state, ..
      } => current_turn_state,
      GameState::Cooperative {
        current_turn_state, ..
      } => current_turn_state,
    }
  }

  pub(crate) fn set_current_player_turn_state(&mut self, turn_state: TurnState) {
    match self {
      GameState::Competitive {
        current_turn_state, ..
      } => *current_turn_state = turn_state,
      GameState::Cooperative {
        current_turn_state, ..
      } => *current_turn_state = turn_state,
    }
  }

  pub fn current_player(&self) -> u8 {
    match self {
      GameState::Competitive { current_player, .. } => *current_player,
      GameState::Cooperative { current_player, .. } => *current_player,
    }
  }

  pub fn current_player_score(&self) -> Option<u32> {
    match self {
      GameState::Competitive {
        current_player,
        player_scores,
        ..
      } => player_scores.get(*current_player as usize).copied(),
      GameState::Cooperative {
        current_player,
        player_scores,
        ..
      } => player_scores.get(*current_player as usize).copied(),
    }
  }

  pub(crate) fn increment_player_count(&mut self) {
    match self {
      GameState::Competitive { player_count, .. } => *player_count += 1,
      GameState::Cooperative { player_count, .. } => *player_count += 1,
    }
  }

  pub(crate) fn advance_turn(&mut self) {
    let next_player = self.current_player() + 1;
    if next_player >= self.player_count() {
      self.set_current_player(0);
    } else {
      self.set_current_player(next_player);
    }
    self.increment_current_player_turn();
  }

  pub(crate) fn increment_current_player_turn(&mut self) {
    match self {
      GameState::Competitive {
        current_player,
        player_turns,
        ..
      } => player_turns[*current_player as usize] += 1,
      GameState::Cooperative {
        current_player,
        player_turns,
        ..
      } => player_turns[*current_player as usize] += 1,
    }
  }

  pub(crate) fn set_current_player(&mut self, player_index: u8) {
    match self {
      GameState::Competitive { current_player, .. } => *current_player = player_index,
      GameState::Cooperative { current_player, .. } => *current_player = player_index,
    }
  }

  pub fn current_player_turn(&self) -> u8 {
    match self {
      GameState::Competitive {
        current_player,
        player_turns,
        ..
      } => player_turns[*current_player as usize],
      GameState::Cooperative {
        current_player,
        player_turns,
        ..
      } => player_turns[*current_player as usize],
    }
  }

  pub fn player_count(&self) -> u8 {
    match self {
      GameState::Competitive { player_count, .. } => *player_count,
      GameState::Cooperative { player_count, .. } => *player_count,
    }
  }

  pub fn max_players(&self) -> u8 {
    match self {
      GameState::Competitive { max_players, .. } => *max_players,
      GameState::Cooperative { max_players, .. } => *max_players,
    }
  }

  pub fn current_player_multiplier(&self) -> f32 {
    match self {
      GameState::Competitive {
        current_player,
        player_multiplier,
        ..
      } => player_multiplier[*current_player as usize],
      GameState::Cooperative {
        current_player,
        player_multiplier,
        ..
      } => player_multiplier[*current_player as usize],
    }
  }

  pub fn current_player_multiplier_mut(&mut self) -> Option<&mut f32> {
    match self {
      GameState::Competitive {
        current_player,
        player_multiplier,
        ..
      } => player_multiplier.get_mut(*current_player as usize),
      GameState::Cooperative {
        current_player,
        player_multiplier,
        ..
      } => player_multiplier.get_mut(*current_player as usize),
    }
  }

  pub fn players_on_team(&self, team_index: u8) -> Option<Vec<u8>> {
    match self {
      GameState::Competitive { .. } => None,
      GameState::Cooperative {
        player_team_map, ..
      } => {
        let players_on_team = player_team_map
          .iter()
          .filter(|(_, team)| *team == &team_index)
          .map(|(&player_index, _)| player_index)
          .collect::<Vec<u8>>();
        Some(players_on_team)
      }
    }
  }

  pub fn player_score(&self, player_index: u8) -> Option<&u32> {
    match self {
      GameState::Competitive { player_scores, .. } => player_scores.get(player_index as usize),
      GameState::Cooperative { player_scores, .. } => player_scores.get(player_index as usize),
    }
  }

  pub(crate) fn player_score_mut(&mut self, player_index: u8) -> Option<&mut u32> {
    match self {
      GameState::Competitive { player_scores, .. } => player_scores.get_mut(player_index as usize),
      GameState::Cooperative { player_scores, .. } => player_scores.get_mut(player_index as usize),
    }
  }

  pub fn current_team(&self) -> Option<u8> {
    match self {
      GameState::Competitive { .. } => None,
      GameState::Cooperative { current_team, .. } => Some(*current_team),
    }
  }

  pub fn team_score(&self, team_index: u8) -> Option<u32> {
    match self {
      GameState::Competitive { .. } => None,
      GameState::Cooperative {
        player_team_map,
        player_scores,
        ..
      } => {
        let team_score: u32 = player_team_map
          .iter()
          .filter(|(_, team)| *team == &team_index)
          .filter_map(|(&player_index, _)| player_scores.get(player_index as usize))
          .sum();
        Some(team_score)
      }
    }
  }
}
