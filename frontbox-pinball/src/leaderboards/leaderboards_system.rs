use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, SubsecRound, Utc};
use frontbox_core::prelude::*;
use frontbox_core::toml;

use crate::{
  GameEndReason, GameEnding, GameStarting, Leaderboard, LeaderboardDefinition, LeaderboardValue,
};

/// Keeps the machine's leaderboards, saved to a file. See [leaderboards](crate::leaderboards).
pub struct LeaderboardsSystem {
  path: PathBuf,
  /// In the order the app listed them, which is the order they're shown in
  boards: Vec<LeaderboardDefinition>,
  /// The key of the board that takes each player's score when a game ends
  final_scores: Option<&'static str>,
  saved: SavedLeaderboards,
  /// What players have earned in the game in progress, not yet saved
  submissions: Vec<Submission>,
}

impl LeaderboardsSystem {
  pub fn new(path: impl AsRef<Path>) -> Self {
    let path = path.as_ref().to_path_buf();
    Self {
      saved: toml::load(&path).unwrap_or_default(),
      path,
      boards: Vec::new(),
      final_scores: None,
      submissions: Vec::new(),
    }
  }

  /// Add a board. Boards are shown in the order they're added.
  pub fn board<T: LeaderboardValue>(mut self, board: &Leaderboard<T>) -> Self {
    let definition = board.definition();
    assert!(
      self.boards.iter().all(|b| b.key != definition.key),
      "leaderboard key `{}` is used by more than one board",
      definition.key
    );
    self.boards.push(definition.clone());
    self
  }

  /// Add the board that takes each player's score when a game ends (`GameEnding`), without anything submitting it
  pub fn final_scores(mut self, board: &Leaderboard<u64>) -> Self {
    self.final_scores = Some(board.definition().key);
    self.board(board)
  }

  /// Every board, in the order they're shown
  pub fn boards(&self) -> &[LeaderboardDefinition] {
    &self.boards
  }

  /// A board as it stands, best first: what players have earned ranked with the board's defaults. Submissions from
  /// the game in progress aren't on it until their initials are entered.
  pub fn standings(&self, board: &LeaderboardDefinition) -> Vec<Standing> {
    self
      .ranked(board, &[])
      .into_iter()
      .enumerate()
      .map(|(rank, ranked)| Standing {
        rank,
        name: ranked.name,
        value: ranked.value,
        at: ranked.at,
      })
      .collect()
  }

  /// [`standings`](Self::standings), with each value as the board's type
  pub fn entries<T: LeaderboardValue>(&self, board: &Leaderboard<T>) -> Vec<LeaderboardEntry<T>> {
    self
      .standings(board.definition())
      .into_iter()
      .map(|standing| LeaderboardEntry {
        rank: standing.rank,
        name: standing.name,
        value: T::from_raw(standing.value),
        at: standing.at,
      })
      .collect()
  }

  /// Offer a value a player (0 for player 1) earned for a board. Only the player's best for the board is kept, so a
  /// running count can be submitted each time it goes up. It's held until the game ends: see
  /// [`placements`](Self::placements).
  pub fn submit<T: LeaderboardValue>(&mut self, board: &Leaderboard<T>, player: u8, value: T) {
    let definition = board.definition();
    let Some(registered) = self.boards.iter().find(|b| b.key == definition.key) else {
      log::error!(
        target: "frontbox::leaderboards",
        "Ignoring a submission to `{}`, which wasn't added to the leaderboards",
        definition.key
      );
      return;
    };
    if registered.type_name != definition.type_name {
      log::error!(
        target: "frontbox::leaderboards",
        "Ignoring a {} submitted to `{}`, which holds {}",
        definition.type_name,
        definition.key,
        registered.type_name
      );
      return;
    }

    let value = value.to_raw();
    let held = self
      .submissions
      .iter_mut()
      .find(|s| s.key == definition.key && s.player == player);
    match held {
      Some(held) => {
        if registered.order(value, held.value).is_lt() {
          held.value = value;
        }
      }
      None => self.submissions.push(Submission {
        key: definition.key,
        player,
        value,
      }),
    }
  }

  /// The boards made in the game in progress: every player's submissions ranked together against each board as it
  /// stands, in board order. A value has to beat an entry to rank above it. Each player in the result enters their
  /// initials once, with [`enter_initials`](Self::enter_initials).
  ///
  /// Players who tie on a board in the same game are ranked in player order here, but end up in the order their
  /// initials are entered.
  pub fn placements(&self) -> Vec<Placement> {
    let mut placements = Vec::new();
    for board in &self.boards {
      let mut candidates: Vec<(u8, u64)> = self
        .submissions
        .iter()
        .filter(|s| s.key == board.key)
        .map(|s| (s.player, s.value))
        .collect();
      candidates.sort_by_key(|(player, _)| *player);

      for (rank, ranked) in self.ranked(board, &candidates).into_iter().enumerate() {
        if let Some(player) = ranked.candidate {
          placements.push(Placement {
            board: board.clone(),
            player,
            rank,
            value: ranked.value,
          });
        }
      }
    }
    placements
  }

  /// Put a player's initials on every board they made this game, and save. Returns what they made, as in
  /// [`placements`](Self::placements). The rest of what they submitted is dropped.
  pub fn enter_initials(&mut self, player: u8, initials: &str) -> Vec<Placement> {
    let made: Vec<Placement> = self
      .placements()
      .into_iter()
      .filter(|p| p.player == player)
      .collect();
    self.submissions.retain(|s| s.player != player);

    let at = Utc::now().trunc_subsecs(0);
    for placement in &made {
      let board = &placement.board;
      let earned = self.saved.boards.entry(board.key.to_string()).or_default();
      earned.entries.push(SavedEntry {
        name: initials.to_string(),
        value: placement.value,
        at: Some(at),
      });
      // stable, so on a tie the older entry stays ahead
      earned.entries.sort_by(|a, b| board.order(a.value, b.value));
      earned.entries.truncate(board.capacity);
      log::info!(
        target: "frontbox::leaderboards",
        "{initials} is {} on {} with {}",
        board.rank_title(placement.rank),
        board.label,
        board.format(placement.value)
      );
    }

    if !made.is_empty() {
      self.write();
    }
    made
  }

  /// The best `capacity` of a board's earned entries, its defaults, and `candidates` (player and value). Where values
  /// tie, earned entries stay ahead of defaults, and both ahead of candidates.
  fn ranked(&self, board: &LeaderboardDefinition, candidates: &[(u8, u64)]) -> Vec<Ranked> {
    let earned = self.saved.boards.get(board.key);
    let earned = earned
      .iter()
      .flat_map(|saved| &saved.entries)
      .map(|e| Ranked {
        name: e.name.clone(),
        value: e.value,
        at: e.at,
        candidate: None,
      });
    let defaults = board.defaults.iter().map(|(name, value)| Ranked {
      name: name.to_string(),
      value: *value,
      at: None,
      candidate: None,
    });
    let candidates = candidates.iter().map(|(player, value)| Ranked {
      name: String::new(),
      value: *value,
      at: None,
      candidate: Some(*player),
    });

    let mut ranked: Vec<Ranked> = earned.chain(defaults).chain(candidates).collect();
    ranked.sort_by(|a, b| board.order(a.value, b.value));
    ranked.truncate(board.capacity);
    ranked
  }

  fn on_game_ending(&mut self, game: &GameEnding) {
    if game.reason == GameEndReason::SlamTilted {
      log::info!(target: "frontbox::leaderboards", "Dropping this game's submissions after a slam tilt");
      self.submissions.clear();
      return;
    }

    let Some(board) = self.final_scores else {
      return;
    };
    for (player, score) in game.scores.iter().enumerate() {
      match self
        .submissions
        .iter_mut()
        .find(|s| s.key == board && s.player == player as u8)
      {
        Some(held) => held.value = held.value.max(*score),
        None => self.submissions.push(Submission {
          key: board,
          player: player as u8,
          value: *score,
        }),
      }
    }
  }

  fn write(&self) {
    if let Err(e) = toml::save(&self.path, &self.saved) {
      log::error!(
        target: "frontbox::leaderboards",
        "Failed to save leaderboards to {}: {e}",
        self.path.display()
      );
    }
  }
}

impl System for LeaderboardsSystem {
  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if event.is::<RoutineStarted<GameStarting>>() {
      // left over when the last game's initials were never entered
      self.submissions.clear();
    } else if let Some(started) = event.downcast_ref::<RoutineStarted<GameEnding>>() {
      self.on_game_ending(&started.routine);
    }
  }

  fn description() -> Option<&'static str> {
    Some("Keeps the high scores and other leaderboards, saved to a file")
  }
}

/// One rank of a board as it stands, from [`LeaderboardsSystem::standings`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
  /// Counting from 0 for the best. See [`LeaderboardDefinition::rank_title`].
  pub rank: usize,
  pub name: String,
  /// The raw value. Show it with [`LeaderboardDefinition::format`].
  pub value: u64,
  /// When the initials were entered. `None` for one of the board's defaults.
  pub at: Option<DateTime<Utc>>,
}

/// One rank of a board as it stands, with its value as the board's type. From [`LeaderboardsSystem::entries`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaderboardEntry<T> {
  /// Counting from 0 for the best
  pub rank: usize,
  pub name: String,
  pub value: T,
  /// When the initials were entered. `None` for one of the board's defaults.
  pub at: Option<DateTime<Utc>>,
}

/// A rank a player earned in the game in progress, waiting for their initials
#[derive(Debug, Clone)]
pub struct Placement {
  pub board: LeaderboardDefinition,
  /// 0 for player 1
  pub player: u8,
  /// Counting from 0 for the best. See [`LeaderboardDefinition::rank_title`].
  pub rank: usize,
  /// The raw value. Show it with [`LeaderboardDefinition::format`].
  pub value: u64,
}

/// A player's best value for a board in the game in progress
struct Submission {
  key: &'static str,
  player: u8,
  value: u64,
}

/// An entry while a board is being ranked
struct Ranked {
  name: String,
  value: u64,
  at: Option<DateTime<Utc>>,
  /// The player whose submission this is, when it isn't on the board yet
  candidate: Option<u8>,
}

/// The file: what players have earned, by board key. Boards the app no longer lists are kept.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct SavedLeaderboards {
  boards: BTreeMap<String, SavedBoard>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct SavedBoard {
  /// Best first, and no more than the board keeps
  entries: Vec<SavedEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SavedEntry {
  name: String,
  value: u64,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use super::*;

  static SCORES: LazyLock<Leaderboard<u64>> = LazyLock::new(|| {
    Leaderboard::highest("High Scores", 3)
      .key("scores")
      .rank_titles(["Grand Champion"])
      .defaults([("AAA", 300), ("BBB", 200), ("CCC", 100)])
  });

  static LOOPS: LazyLock<Leaderboard<u32>> =
    LazyLock::new(|| Leaderboard::highest("Loop Champion", 1).key("loops"));

  static FASTEST: LazyLock<Leaderboard<Duration>> = LazyLock::new(|| {
    Leaderboard::lowest("Fastest", 1)
      .key("fastest")
      .defaults([("AAA", Duration::from_secs(60))])
  });

  /// A path in a fresh, empty directory per test
  fn test_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
      "frontbox-leaderboards-{}-{name}",
      std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("leaderboards.toml")
  }

  fn system(path: &Path) -> LeaderboardsSystem {
    LeaderboardsSystem::new(path)
      .final_scores(&SCORES)
      .board(&LOOPS)
      .board(&FASTEST)
  }

  fn names(boards: &LeaderboardsSystem, board: &LeaderboardDefinition) -> Vec<String> {
    let standings = boards.standings(board);
    standings.into_iter().map(|s| s.name).collect()
  }

  /// (board key, player, rank)
  fn placed(placements: &[Placement]) -> Vec<(&'static str, u8, usize)> {
    let placed = placements.iter().map(|p| (p.board.key, p.player, p.rank));
    placed.collect()
  }

  #[test]
  fn a_new_board_shows_its_defaults() {
    let boards = system(&test_path("defaults"));

    assert_eq!(names(&boards, SCORES.definition()), ["AAA", "BBB", "CCC"]);
    assert!(boards.standings(LOOPS.definition()).is_empty());
  }

  #[test]
  fn only_a_players_best_submission_is_kept() {
    let mut boards = system(&test_path("best"));
    boards.submit(&LOOPS, 0, 4);
    boards.submit(&LOOPS, 0, 9);
    boards.submit(&LOOPS, 0, 6);
    boards.submit(&FASTEST, 0, Duration::from_secs(50));
    boards.submit(&FASTEST, 0, Duration::from_secs(40));
    boards.submit(&FASTEST, 0, Duration::from_secs(45));

    let placements = boards.placements();
    assert_eq!(placed(&placements), [("loops", 0, 0), ("fastest", 0, 0)]);
    assert_eq!(placements[0].value, 9);
    assert_eq!(placements[1].value, 40_000);
  }

  #[test]
  fn players_are_ranked_together() {
    let mut boards = system(&test_path("together"));
    // both beat CCC, but the board only has room for one more below AAA and BBB
    boards.submit(&SCORES, 0, 150);
    boards.submit(&SCORES, 1, 180);
    boards.submit(&LOOPS, 0, 3);
    boards.submit(&LOOPS, 1, 5);

    assert_eq!(
      placed(&boards.placements()),
      [("scores", 1, 2), ("loops", 1, 0)]
    );
  }

  #[test]
  fn a_value_has_to_beat_an_entry_to_rank_above_it() {
    let mut boards = system(&test_path("ties"));
    boards.submit(&SCORES, 0, 100);
    boards.submit(&FASTEST, 0, Duration::from_secs(60));
    assert!(boards.placements().is_empty());

    boards.submit(&SCORES, 0, 300);
    assert_eq!(placed(&boards.placements()), [("scores", 0, 1)]);
  }

  #[test]
  fn initials_go_on_every_board_the_player_made_and_are_saved() {
    let path = test_path("initials");
    let mut boards = system(&path);
    boards.submit(&SCORES, 0, 500);
    boards.submit(&LOOPS, 0, 7);
    boards.submit(&LOOPS, 1, 2);

    let made = boards.enter_initials(0, "TMS");
    assert_eq!(placed(&made), [("scores", 0, 0), ("loops", 0, 0)]);
    assert!(boards.enter_initials(1, "XXX").is_empty());
    assert!(boards.placements().is_empty());

    let loaded = system(&path);
    assert_eq!(names(&loaded, SCORES.definition()), ["TMS", "AAA", "BBB"]);
    let loops = loaded.entries(&LOOPS);
    assert_eq!((loops[0].name.as_str(), loops[0].value), ("TMS", 7));
    assert!(loops[0].at.is_some());
    assert!(loaded.standings(SCORES.definition())[1].at.is_none());
  }

  #[test]
  fn an_older_entry_stays_ahead_on_a_tie() {
    let path = test_path("older");
    let mut boards = system(&path);
    boards.submit(&SCORES, 0, 250);
    assert_eq!(placed(&boards.enter_initials(0, "OLD")), [("scores", 0, 1)]);

    boards.submit(&SCORES, 0, 250);
    assert_eq!(placed(&boards.enter_initials(0, "NEW")), [("scores", 0, 2)]);
    assert_eq!(names(&boards, SCORES.definition()), ["AAA", "OLD", "NEW"]);
  }

  #[test]
  fn final_scores_are_submitted_when_the_game_ends() {
    let context = TestContext::default();
    let mut boards = system(&test_path("final"));
    let ending = |reason| RoutineStarted {
      routine: GameEnding {
        scores: vec![50, 400],
        reason,
      },
    };

    boards.on_event(&ending(GameEndReason::Completed), &context.sys_ctx());
    assert_eq!(placed(&boards.placements()), [("scores", 1, 0)]);

    // never entered: gone by the next game
    let starting = RoutineStarted {
      routine: GameStarting,
    };
    boards.on_event(&starting, &context.sys_ctx());
    assert!(boards.placements().is_empty());

    boards.submit(&LOOPS, 0, 3);
    boards.on_event(&ending(GameEndReason::SlamTilted), &context.sys_ctx());
    assert!(boards.placements().is_empty());
  }

  #[test]
  fn a_submission_to_a_board_that_wasnt_added_is_ignored() {
    let mut boards = LeaderboardsSystem::new(test_path("unknown")).board(&SCORES);
    boards.submit(&LOOPS, 0, 3);
    assert!(boards.placements().is_empty());
  }
}
