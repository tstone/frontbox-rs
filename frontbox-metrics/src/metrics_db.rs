use std::path::Path;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use stoolap::Database;

use crate::{MetricKind, MetricsBatch, MetricsStore, StoreResult};

/// The metrics database: written by the machine as it flushes, and read by anything that wants its history.
///
/// Within a process, every `MetricsDb` opened on a directory shares one database engine, so a reader sees each flush
/// as soon as it's committed. Only one process can open a directory at a time; a tool reading history while the
/// machine runs reads a copy.
///
/// Queries block, so call them from `tokio::task::spawn_blocking` or a thread of their own, never from the run loop.
#[derive(Clone)]
pub struct MetricsDb {
  db: Database,
}

impl MetricsDb {
  /// Opens the database in `dir`, creating it and its tables if missing. Opening the same directory again in this
  /// process returns the same database.
  pub fn open(dir: &Path) -> StoreResult<Self> {
    std::fs::create_dir_all(dir)?;
    // Stoolap shares an engine between opens of the same DSN, and a second engine on the directory would fail on its
    // lock, so this is the only place the DSN is built. Every commit is fsynced so a flushed game survives an
    // immediate power cut; there are only a few a minute.
    let db = Database::open(&format!("file://{}?sync_mode=full", dir.display()))?;
    for table in SCHEMA {
      db.execute(table, ())?;
    }
    Ok(Self { db })
  }

  /// Every counter's total over all time, sorted by category and label
  pub fn totals(&self) -> StoreResult<Vec<CounterTotal>> {
    let rows = self.db.query(
      "SELECT m.metric, m.label, m.description, m.category, COALESCE(SUM(c.count), 0)
        FROM metrics m LEFT JOIN counts c ON c.metric = m.metric
        WHERE m.kind = 'counter'
        GROUP BY m.metric, m.label, m.description, m.category
        ORDER BY m.category, m.label",
      (),
    )?;

    let mut totals = Vec::new();
    for row in rows {
      let row = row?;
      totals.push(CounterTotal {
        metric: row.get(0)?,
        label: row.get(1)?,
        desc: row.get(2)?,
        category: row.get(3)?,
        total: row.get::<i64>(4)? as u64,
      });
    }
    Ok(totals)
  }
}

impl MetricsStore for MetricsDb {
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

/// A counter and everything it has counted
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct CounterTotal {
  pub metric: String,
  pub label: String,
  pub desc: String,
  pub category: Option<String>,
  #[cfg_attr(feature = "ts", ts(type = "number"))]
  pub total: u64,
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
  use std::path::PathBuf;

  use super::*;
  use crate::{Counter, MetricsCollection, Sample};

  #[tokio::test]
  async fn flushes_are_kept_across_reopening() {
    let dir = TempDir::new("reopen");

    let hits = Counter::new("hits", "Times hit").category("Switches");
    let score = Sample::<u32>::new("score", "");
    let metrics = MetricsCollection::default();
    metrics.begin_store_task(MetricsDb::open(&dir.0).unwrap());

    metrics.add(&hits, 2);
    metrics.begin_game();
    metrics.set_player(1);
    metrics.inc(&hits);
    metrics.flush(); // as at a turn's end
    metrics.record(&score, 500);
    metrics.end_game();
    metrics.inc(&hits);
    metrics.close().await; // the store task has dropped its handle, and with it the engine

    let store = MetricsDb::open(&dir.0).unwrap();
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
  }

  #[test]
  fn a_redeclared_metric_replaces_its_definition() {
    let dir = TempDir::new("redeclare");
    let mut store = MetricsDb::open(&dir.0).unwrap();

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
  }

  #[test]
  fn totals_sum_every_flush_of_each_counter() {
    let dir = TempDir::new("totals");
    let mut store = MetricsDb::open(&dir.0).unwrap();

    let sling = Counter::new("left_sling", "").key("switch_hits.left_sling").category("Switches");
    let saves = Counter::new("Ball Saves", "Drains saved");
    let unused = Counter::new("Tilts", "");
    let metrics = MetricsCollection::default();
    metrics.add(&sling, 3);
    metrics.inc(&saves);
    metrics.add(&unused, 0); // defined, never counted
    metrics.record(&Sample::<u32>::new("score", ""), 10); // samples have no total
    store.write(&metrics.take_batch()).unwrap();
    metrics.add(&sling, 2);
    store.write(&metrics.take_batch()).unwrap();

    let mut totals: Vec<_> = store
      .totals()
      .unwrap()
      .into_iter()
      .map(|t| (t.metric, t.category, t.total))
      .collect();
    totals.sort(); // where uncategorized metrics sort is up to the database
    assert_eq!(
      totals,
      vec![
        ("Ball Saves".into(), None, 1),
        ("Tilts".into(), None, 0),
        ("switch_hits.left_sling".into(), Some("Switches".into()), 5),
      ]
    );
  }

  #[test]
  fn a_reader_sees_writes_through_another_handle() {
    let dir = TempDir::new("shared");
    let mut writer = MetricsDb::open(&dir.0).unwrap();
    let reader = MetricsDb::open(&dir.0).unwrap(); // as the console would, while the machine runs

    let metrics = MetricsCollection::default();
    metrics.inc(&Counter::new("hits", ""));
    writer.write(&metrics.take_batch()).unwrap();

    assert_eq!(reader.totals().unwrap()[0].total, 1);
  }

  /// A fresh directory for a test's database, removed afterwards
  struct TempDir(PathBuf);

  impl TempDir {
    fn new(name: &str) -> Self {
      let dir = std::env::temp_dir().join(format!("frontbox-metrics-{name}-{}", std::process::id()));
      let _ = std::fs::remove_dir_all(&dir);
      Self(dir)
    }
  }

  impl Drop for TempDir {
    fn drop(&mut self) {
      let _ = std::fs::remove_dir_all(&self.0);
    }
  }
}
