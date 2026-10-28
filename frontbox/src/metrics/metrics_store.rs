use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use stoolap::Database;

use crate::metrics::{MetricKind, MetricsBatch};

pub(crate) type StoreResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Where flushed metrics are kept. Writes are blocking, so they run off the run loop on the store task.
pub(crate) trait MetricsStore: Send + 'static {
  /// Writes a batch as one transaction: all of it or none of it
  fn write(&mut self, batch: &MetricsBatch) -> StoreResult<()>;
}

/// Metrics in an embedded Stoolap database
pub(crate) struct StoolapStore {
  db: Database,
}

impl StoolapStore {
  /// Opens the database in `dir`, creating it and its tables if missing
  pub fn open(dir: &Path) -> StoreResult<Self> {
    std::fs::create_dir_all(dir)?;
    // every commit is fsynced so a flushed game survives an immediate power cut; there are only a few per game
    let db = Database::open(&format!("file://{}?sync_mode=full", dir.display()))?;
    for table in SCHEMA {
      db.execute(table, ())?;
    }
    Ok(Self { db })
  }
}

impl MetricsStore for StoolapStore {
  fn write(&mut self, batch: &MetricsBatch) -> StoreResult<()> {
    let mut tx = self.db.begin()?;

    for definition in &batch.definitions {
      let (kind, sample_type) = match definition.kind {
        MetricKind::Counter => ("counter", None),
        MetricKind::Sample { type_name } => ("sample", Some(type_name)),
      };
      tx.execute(
        "INSERT INTO metrics (metric, kind, sample_type, label, description, category)
          VALUES ($1, $2, $3, $4, $5, $6)
          ON CONFLICT (metric) DO UPDATE SET
            kind = EXCLUDED.kind, sample_type = EXCLUDED.sample_type, label = EXCLUDED.label,
            description = EXCLUDED.description, category = EXCLUDED.category",
        (
          definition.key,
          kind,
          sample_type,
          definition.label,
          definition.desc,
          definition.category,
        ),
      )?;
    }

    for game in &batch.games {
      tx.execute(
        "INSERT INTO games (id, started_at, ended_at, player_count) VALUES ($1, $2, $3, $4)",
        (
          game.id.0 as i64,
          timestamp(game.started_at),
          timestamp(game.ended_at),
          game.player_count,
        ),
      )?;
    }

    let flushed_at = timestamp(batch.taken_at);
    for delta in &batch.counts {
      tx.execute(
        "INSERT INTO counts (metric, count, game_id, flushed_at) VALUES ($1, $2, $3, $4)",
        (
          delta.metric,
          delta.count as i64,
          delta.game.map(|game| game.0 as i64),
          flushed_at,
        ),
      )?;
    }

    for sample in &batch.samples {
      tx.execute(
        "INSERT INTO samples (metric, value, game_id, player, recorded_at) VALUES ($1, $2, $3, $4, $5)",
        (
          sample.metric,
          sample.value,
          sample.game.map(|game| game.0 as i64),
          sample.player,
          timestamp(sample.recorded_at),
        ),
      )?;
    }

    tx.commit()?;
    Ok(())
  }
}

const SCHEMA: [&str; 4] = [
  "CREATE TABLE IF NOT EXISTS metrics (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    metric TEXT NOT NULL UNIQUE,
    kind TEXT NOT NULL,
    sample_type TEXT,
    label TEXT NOT NULL,
    description TEXT NOT NULL,
    category TEXT
  )",
  "CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY,
    started_at TIMESTAMP NOT NULL,
    ended_at TIMESTAMP NOT NULL,
    player_count INTEGER NOT NULL
  )",
  "CREATE TABLE IF NOT EXISTS counts (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    metric TEXT NOT NULL,
    count INTEGER NOT NULL,
    game_id INTEGER,
    flushed_at TIMESTAMP NOT NULL
  )",
  "CREATE TABLE IF NOT EXISTS samples (
    id INTEGER PRIMARY KEY AUTO_INCREMENT,
    metric TEXT NOT NULL,
    value FLOAT NOT NULL,
    game_id INTEGER,
    player INTEGER,
    recorded_at TIMESTAMP NOT NULL
  )",
];

fn timestamp(time: SystemTime) -> DateTime<Utc> {
  time.into()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::metrics::{Counter, MetricsCollection, Sample};

  #[tokio::test]
  async fn flushes_are_kept_across_reopening() {
    let dir = std::env::temp_dir().join(format!("frontbox-metrics-store-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);

    let hits = Counter::new("hits", "Times hit").category("Switches");
    let score = Sample::<u32>::new("score", "");
    let metrics = MetricsCollection::default();
    metrics.begin_store_task(StoolapStore::open(&dir).unwrap());

    metrics.add(&hits, 2);
    metrics.begin_game();
    metrics.set_player(1);
    metrics.inc(&hits);
    metrics.flush(); // as at a turn's end
    metrics.record(&score, 500);
    metrics.end_game();
    metrics.inc(&hits);
    metrics.close().await; // the store task owned the database, and has dropped it

    let store = StoolapStore::open(&dir).unwrap();
    let total = |sql: &str| -> i64 {
      let row = store.db.query(sql, ()).unwrap().next().unwrap().unwrap();
      row.get::<i64>(0).unwrap()
    };
    assert_eq!(total("SELECT SUM(count) FROM counts WHERE metric = 'hits'"), 4);
    assert_eq!(total("SELECT COUNT(*) FROM counts WHERE game_id IS NOT NULL"), 1);
    assert_eq!(total("SELECT COUNT(*) FROM metrics"), 2);
    assert_eq!(total("SELECT player_count FROM games"), 2);
    assert_eq!(total("SELECT player FROM samples WHERE metric = 'score'"), 1);

    let row = store
      .db
      .query("SELECT kind, category FROM metrics WHERE metric = 'hits'", ())
      .unwrap()
      .next()
      .unwrap()
      .unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "counter");
    assert_eq!(row.get::<String>(1).unwrap(), "Switches");

    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
  }

  #[test]
  fn a_redeclared_metric_replaces_its_definition() {
    let dir = std::env::temp_dir().join(format!("frontbox-metrics-redeclare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut store = StoolapStore::open(&dir).unwrap();

    let before = MetricsCollection::default();
    before.inc(&Counter::new("Drains", "old"));
    store.write(&before.take_batch()).unwrap();
    let after = MetricsCollection::default(); // as on the next boot
    after.inc(&Counter::new("Drains", "new"));
    store.write(&after.take_batch()).unwrap();

    let row = store
      .db
      .query("SELECT description FROM metrics", ())
      .unwrap()
      .next()
      .unwrap()
      .unwrap();
    assert_eq!(row.get::<String>(0).unwrap(), "new");

    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
  }
}
