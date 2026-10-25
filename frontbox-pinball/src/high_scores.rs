use std::collections::HashMap;
use std::path::Path;

use frontbox::prelude::System;
use frontbox::toml;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HighScoresSystem {
  high_scores: Vec<ScoreEntry>,
  custom_scores: HashMap<String, ScoreEntry>,
  #[serde(skip)]
  path: String,
}

impl HighScoresSystem {
  pub fn new(path: impl AsRef<Path>) -> Self {
    let path = path.as_ref();
    let scores: HighScoresSystem = toml::load(path).unwrap_or_default();

    Self {
      high_scores: scores.high_scores,
      custom_scores: scores.custom_scores,
      path: path.to_string_lossy().into_owned(),
    }
  }

  pub fn get_custom_scores(&self) -> &HashMap<String, ScoreEntry> {
    &self.custom_scores
  }

  pub fn set_custom_score(&mut self, key: impl Into<String>, name: String, score: u32) {
    self
      .custom_scores
      .insert(key.into(), ScoreEntry { name, score });
    self.write();
  }

  pub fn get_high_scores(&self) -> &Vec<ScoreEntry> {
    &self.high_scores
  }

  pub fn set_high_score(&mut self, slot: HighScoreSlot, name: String, score: u32) {
    let entry = ScoreEntry { name, score };
    match slot {
      HighScoreSlot::GrandChampion => self.high_scores[0] = entry,
      HighScoreSlot::HighScore1 => self.high_scores[1] = entry,
      HighScoreSlot::HighScore2 => self.high_scores[2] = entry,
      HighScoreSlot::HighScore3 => self.high_scores[3] = entry,
      HighScoreSlot::HighScore4 => self.high_scores[4] = entry,
    }
    self.write();
  }

  /// Check if the given score is a new high score
  pub fn is_high_score(&self, score: u32) -> Option<HighScoreSlot> {
    let scores = (0..4)
      .map(|i| {
        self
          .high_scores
          .get(i)
          .unwrap_or(&ScoreEntry::default())
          .score
      })
      .collect::<Vec<_>>();

    if score > scores[0] {
      Some(HighScoreSlot::GrandChampion)
    } else if score > scores[1] {
      Some(HighScoreSlot::HighScore1)
    } else if score > scores[2] {
      Some(HighScoreSlot::HighScore2)
    } else if score > scores[3] {
      Some(HighScoreSlot::HighScore3)
    } else if score > scores[4] {
      Some(HighScoreSlot::HighScore4)
    } else {
      None
    }
  }

  fn write(&self) {
    if let Err(e) = toml::save(&self.path, self) {
      log::error!("Failed to save high scores to {}: {e}", self.path);
    }
  }
}

impl System for HighScoresSystem {}

impl Default for HighScoresSystem {
  fn default() -> Self {
    Self {
      high_scores: vec![
        ScoreEntry {
          name: "AAA".to_string(),
          score: 50_000_000,
        },
        ScoreEntry {
          name: "BBB".to_string(),
          score: 40_000_000,
        },
        ScoreEntry {
          name: "CCC".to_string(),
          score: 30_000_000,
        },
        ScoreEntry {
          name: "DDD".to_string(),
          score: 20_000_000,
        },
        ScoreEntry {
          name: "EEE".to_string(),
          score: 10_000_000,
        },
      ],
      custom_scores: HashMap::new(),
      path: String::default().into(),
    }
  }
}

pub enum HighScoreSlot {
  GrandChampion,
  HighScore1,
  HighScore2,
  HighScore3,
  HighScore4,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScoreEntry {
  pub name: String,
  pub score: u32,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn saved_scores_load_back() {
    let dir = std::env::temp_dir().join(format!("frontbox-high-scores-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("high_scores.toml");

    let mut scores = HighScoresSystem::new(&path);
    scores.set_high_score(HighScoreSlot::GrandChampion, "ZZZ".to_string(), 99_000_000);
    scores.set_custom_score("loops", "LLL".to_string(), 12);

    let loaded = HighScoresSystem::new(&path);
    assert_eq!(loaded.get_high_scores()[0].name, "ZZZ");
    assert_eq!(loaded.get_high_scores()[1].name, "BBB");
    assert_eq!(loaded.get_custom_scores()["loops"].score, 12);
  }
}
