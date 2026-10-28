use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::time::{Duration, SystemTime};

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::{Counter, MetricDefinition, MetricsStore, Sample, SampleType};

/// A game recording more samples of one metric than this is probably recording a sample where a counter belongs
const SAMPLES_PER_GAME_WARNING: u32 = 100;

/// How often the app flushes recordings whatever the game is doing. Bounds what a power-off loses, and how far
/// history read from the store trails what has been recorded.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(30);

/// How long shutdown waits for the last flush to be written
const CLOSE_TIMEOUT: Duration = Duration::from_secs(5);

/// Everything recorded since the last flush, and the game and player that recordings are tagged with
#[derive(Default)]
pub struct MetricsCollection {
  state: RefCell<State>,
  /// Hands flushed batches to the task writing them to the store, when there's a store
  writer: RefCell<Option<StoreWriter>>,
}

impl MetricsCollection {
  /// Writes to the store should happen outside of the main run loop, so flushed batches are handed to a tokio task
  /// that owns the store and writes them one at a time
  pub fn begin_store_task<S: MetricsStore>(&self, mut store: S) {
    let (sender, mut receiver) = mpsc::unbounded_channel::<MetricsBatch>();
    let task = tokio::spawn(async move {
      while let Some(batch) = receiver.recv().await {
        let written = tokio::task::spawn_blocking(move || {
          let result = store.write(&batch);
          (store, result)
        })
        .await;
        match written {
          Ok((returned, result)) => {
            store = returned;
            if let Err(e) = result {
              log::error!("Failed to write metrics: {e}");
            }
          }
          Err(e) => {
            log::error!("Metrics store panicked; no more metrics will be saved: {e}");
            return;
          }
        }
      }
    });
    *self.writer.borrow_mut() = Some(StoreWriter { sender, task });
  }

  /// Hand everything recorded since the last flush to the store. Without a store it's dropped.
  ///
  /// A game's end flushes on its own, and a timer flushes every few minutes. Call this at other natural pauses, such
  /// as the end of a turn, so less is lost when the machine is switched off.
  pub fn flush(&self) {
    let batch = self.take_batch();
    if batch.is_empty() {
      return;
    }
    if let Some(writer) = &*self.writer.borrow() {
      let _ = writer.sender.send(batch);
    }
  }

  /// Flush, then wait for the store to finish writing. Called at shutdown.
  pub async fn close(&self) {
    self.flush();
    let Some(writer) = self.writer.borrow_mut().take() else {
      return;
    };
    drop(writer.sender); // the task ends once it has written what's queued
    if tokio::time::timeout(CLOSE_TIMEOUT, writer.task).await.is_err() {
      log::error!("Timed out writing the last metrics at shutdown");
    }
  }

  /// Count one occurrence
  pub fn inc(&self, counter: &Counter) {
    self.add(counter, 1);
  }

  /// Count several occurrences at once
  pub fn add(&self, counter: &Counter, count: u64) {
    let definition = counter.definition();
    let mut state = self.state.borrow_mut();
    if !state.define(definition) {
      return;
    }
    let key = CountKey {
      metric: definition.key,
      game: state.game.as_ref().map(|game| game.id),
    };
    *state.counts.entry(key).or_default() += count;
  }

  /// Record a value, tagged with the game and player in progress if any
  pub fn record<T: SampleType>(&self, sample: &Sample<T>, value: T) {
    let definition = sample.definition();
    let mut state = self.state.borrow_mut();
    if !state.define(definition) {
      return;
    }

    let game = state.game.as_mut().map(|game| {
      let recorded = game.samples_per_metric.entry(definition.key).or_default();
      *recorded += 1;
      if *recorded == SAMPLES_PER_GAME_WARNING {
        log::warn!(
          "metric '{}' has recorded {SAMPLES_PER_GAME_WARNING} samples this game; a `Counter` may suit it better",
          definition.key
        );
      }
      (game.id, game.player)
    });

    state.samples.push(SampleRecord {
      metric: definition.key,
      value: value.to_sample(),
      game: game.map(|(id, _)| id),
      player: game.and_then(|(_, player)| player),
      recorded_at: SystemTime::now(),
    });
  }

  /// Start tagging recordings with a new game. A game still in progress is ended first.
  pub fn begin_game(&self) {
    let mut state = self.state.borrow_mut();
    if state.game.is_some() {
      log::warn!("a game began before the previous one ended; ending it now");
      state.end_game();
    }

    let started_at = SystemTime::now();
    // ids are start times, nudged forward so two games started in the same millisecond stay distinct
    let millis = started_at
      .duration_since(SystemTime::UNIX_EPOCH)
      .map(|elapsed| elapsed.as_millis() as u64)
      .unwrap_or_default();
    let id = GameId(millis.max(state.last_game_id.0 + 1));
    state.last_game_id = id;

    state.game = Some(GameInProgress {
      id,
      started_at,
      player: None,
      player_count: 0,
      samples_per_metric: HashMap::new(),
    });
  }

  /// Tag recordings with the player whose turn it is, counting from 0. Ignored outside a game.
  pub fn set_player(&self, player: u8) {
    if let Some(game) = &mut self.state.borrow_mut().game {
      game.player = Some(player);
      game.player_count = game.player_count.max(player + 1);
    }
  }

  /// Stop tagging recordings with the game in progress, and flush
  pub fn end_game(&self) {
    self.state.borrow_mut().end_game();
    self.flush();
  }

  /// Takes everything recorded since the last batch, as a flush does. For writing to a store by hand, and for tests.
  pub fn take_batch(&self) -> MetricsBatch {
    let mut state = self.state.borrow_mut();
    MetricsBatch {
      taken_at: SystemTime::now(),
      definitions: std::mem::take(&mut state.new_definitions),
      counts: state
        .counts
        .drain()
        .map(|(key, count)| CountDelta {
          metric: key.metric,
          game: key.game,
          count,
        })
        .collect(),
      samples: std::mem::take(&mut state.samples),
      games: std::mem::take(&mut state.games),
    }
  }
}

/// A game's identity: its start time in milliseconds since the Unix epoch
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameId(pub u64);

/// Everything recorded since the previous batch
#[derive(Clone, Debug)]
pub struct MetricsBatch {
  pub taken_at: SystemTime,
  /// Metrics first used since the previous batch
  pub definitions: Vec<MetricDefinition>,
  pub counts: Vec<CountDelta>,
  pub samples: Vec<SampleRecord>,
  /// Games that ended
  pub games: Vec<GameRecord>,
}

impl MetricsBatch {
  pub fn is_empty(&self) -> bool {
    self.definitions.is_empty()
      && self.counts.is_empty()
      && self.samples.is_empty()
      && self.games.is_empty()
  }
}

/// How much a counter went up since the previous batch
#[derive(Clone, Debug, PartialEq)]
pub struct CountDelta {
  pub metric: &'static str,
  pub game: Option<GameId>,
  pub count: u64,
}

#[derive(Clone, Debug)]
pub struct SampleRecord {
  pub metric: &'static str,
  pub value: f64,
  pub game: Option<GameId>,
  pub player: Option<u8>,
  pub recorded_at: SystemTime,
}

#[derive(Clone, Debug)]
pub struct GameRecord {
  pub id: GameId,
  pub started_at: SystemTime,
  pub ended_at: SystemTime,
  pub player_count: u8,
}

#[derive(Default)]
struct State {
  /// The first declaration seen for each key
  definitions: HashMap<&'static str, MetricDefinition>,
  /// Declarations first seen since the previous batch, for the store to label history with
  new_definitions: Vec<MetricDefinition>,
  counts: HashMap<CountKey, u64>,
  samples: Vec<SampleRecord>,
  games: Vec<GameRecord>,
  game: Option<GameInProgress>,
  last_game_id: GameId,
}

impl State {
  /// Remembers a metric on first use. Whether it can be recorded: a definition that conflicts with the first one seen
  /// for its key can't be, since its values would be read as the other's.
  fn define(&mut self, definition: &MetricDefinition) -> bool {
    let first = match self.definitions.entry(definition.key) {
      Entry::Occupied(entry) => entry.into_mut(),
      Entry::Vacant(entry) => {
        self.new_definitions.push(definition.clone());
        entry.insert(definition.clone())
      }
    };
    if first.is_compatible(definition) {
      return true;
    }

    let message = format!(
      "metric key '{}' is declared by '{}' and '{}' as different kinds or types",
      definition.key, first.label, definition.label
    );
    if cfg!(debug_assertions) {
      panic!("{message}. Give one a distinct `.key(...)`");
    }
    log::error!(
      "{message}; '{}' is not recorded until fixed",
      definition.label
    );
    false
  }

  fn end_game(&mut self) {
    if let Some(game) = self.game.take() {
      self.games.push(GameRecord {
        id: game.id,
        started_at: game.started_at,
        ended_at: SystemTime::now(),
        player_count: game.player_count,
      });
    }
  }
}

struct StoreWriter {
  sender: mpsc::UnboundedSender<MetricsBatch>,
  task: JoinHandle<()>,
}

#[derive(PartialEq, Eq, Hash)]
struct CountKey {
  metric: &'static str,
  game: Option<GameId>,
}

struct GameInProgress {
  id: GameId,
  started_at: SystemTime,
  player: Option<u8>,
  /// Players who have had a turn
  player_count: u8,
  samples_per_metric: HashMap<&'static str, u32>,
}

#[cfg(test)]
mod tests {
  use std::time::Duration;

  use super::*;
  use crate::StoreResult;

  #[test]
  fn counts_add_up_per_metric() {
    let hits = Counter::new("hits", "");
    let drains = Counter::new("drains", "");
    let metrics = MetricsCollection::default();

    metrics.inc(&hits);
    metrics.add(&hits, 2);
    metrics.inc(&drains);

    let mut counts = metrics.take_batch().counts;
    counts.sort_by_key(|delta| delta.metric);
    let counts: Vec<_> = counts.iter().map(|d| (d.metric, d.count)).collect();
    assert_eq!(counts, vec![("drains", 1), ("hits", 3)]);
  }

  #[test]
  fn a_batch_takes_only_what_was_recorded_since_the_last() {
    let hits = Counter::new("hits", "");
    let metrics = MetricsCollection::default();

    metrics.add(&hits, 5);
    assert_eq!(metrics.take_batch().counts[0].count, 5);
    assert!(metrics.take_batch().counts.is_empty());

    metrics.inc(&hits);
    assert_eq!(metrics.take_batch().counts[0].count, 1);
  }

  #[tokio::test]
  async fn recordings_are_tagged_with_the_game_and_player_in_progress() {
    let hits = Counter::new("hits", "");
    let score = Sample::<u32>::new("score", "");
    let (metrics, store) = with_store();

    metrics.inc(&hits);
    metrics.begin_game();
    metrics.inc(&hits);
    metrics.record(&score, 100);
    metrics.set_player(1);
    metrics.inc(&hits);
    metrics.record(&score, 200);
    metrics.end_game();
    metrics.record(&score, 300);

    let batch = store.everything_written(&metrics).await;
    let game = batch.games[0].id;
    assert_eq!(batch.games[0].player_count, 2);

    let mut counts: Vec<_> = batch.counts.iter().map(|d| (d.game, d.count)).collect();
    counts.sort();
    assert_eq!(counts, vec![(None, 1), (Some(game), 2)]);

    let samples: Vec<_> = batch
      .samples
      .iter()
      .map(|s| (s.value, s.game, s.player))
      .collect();
    assert_eq!(
      samples,
      vec![
        (100.0, Some(game), None),
        (200.0, Some(game), Some(1)),
        (300.0, None, None),
      ]
    );
  }

  #[tokio::test]
  async fn games_started_together_get_distinct_ids() {
    let (metrics, store) = with_store();
    metrics.begin_game();
    metrics.begin_game(); // ends the first
    metrics.end_game();

    let games = store.everything_written(&metrics).await.games;
    assert_eq!(games.len(), 2);
    assert!(games[0].id < games[1].id);
  }

  #[tokio::test]
  async fn a_game_is_written_when_it_ends() {
    let hits = Counter::new("hits", "");
    let (metrics, store) = with_store();

    metrics.begin_game();
    metrics.inc(&hits);
    metrics.end_game();
    metrics.inc(&hits); // after the game, so not part of its flush

    let written = tokio::time::timeout(Duration::from_secs(2), async {
      loop {
        if let Some(batch) = store.0.lock().unwrap().first() {
          break (batch.games.len(), batch.counts.len());
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
      }
    })
    .await
    .expect("the game was never written");
    assert_eq!(written, (1, 1));
  }

  #[tokio::test]
  async fn definitions_are_written_once() {
    let hits = Counter::new("hits", "");
    let (metrics, store) = with_store();

    metrics.inc(&hits);
    metrics.flush();
    metrics.inc(&hits);
    metrics.flush();
    metrics.close().await;

    let batches = store.0.lock().unwrap();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0].definitions.len(), 1);
    assert!(batches[1].definitions.is_empty());
  }

  #[tokio::test]
  async fn nothing_recorded_writes_nothing() {
    let (metrics, store) = with_store();
    metrics.flush();
    metrics.close().await;
    assert!(store.0.lock().unwrap().is_empty());
  }

  #[test]
  fn without_a_store_a_flush_drops_what_was_recorded() {
    let hits = Counter::new("hits", "");
    let metrics = MetricsCollection::default();

    metrics.inc(&hits);
    metrics.flush();

    assert!(metrics.take_batch().counts.is_empty());
  }

  #[test]
  fn players_are_ignored_outside_a_game() {
    let score = Sample::<u32>::new("score", "");
    let metrics = MetricsCollection::default();

    metrics.set_player(2);
    metrics.record(&score, 1);

    assert_eq!(metrics.take_batch().samples[0].player, None);
  }

  #[test]
  fn durations_are_stored_as_milliseconds() {
    let length = Sample::<Duration>::new("length", "");
    let metrics = MetricsCollection::default();

    metrics.record(&length, Duration::from_secs(90));

    assert_eq!(metrics.take_batch().samples[0].value, 90_000.0);
    assert_eq!(length.definition().format(90_000.0), "1m 30s");
  }

  #[test]
  fn definitions_on_one_key_with_the_same_kind_share_counts() {
    let first = Counter::new("Drains", "");
    let reworded = Counter::new("Balls Drained", "Another reader").key("Drains");
    let metrics = MetricsCollection::default();

    metrics.inc(&first);
    metrics.inc(&reworded);

    let batch = metrics.take_batch();
    assert_eq!(
      batch.counts,
      vec![CountDelta {
        metric: "Drains",
        game: None,
        count: 2
      }]
    );
    assert_eq!(batch.definitions.len(), 1);
  }

  #[test]
  #[cfg(debug_assertions)]
  #[should_panic(expected = "different kinds or types")]
  fn different_sample_types_on_one_key_conflict() {
    let count = Sample::<u32>::new("length", "");
    let length = Sample::<Duration>::new("length", "");
    let metrics = MetricsCollection::default();

    metrics.record(&count, 3);
    metrics.record(&length, Duration::from_millis(3));
  }

  #[test]
  #[cfg(debug_assertions)]
  #[should_panic(expected = "different kinds or types")]
  fn a_counter_and_a_sample_on_one_key_conflict() {
    let counter = Counter::new("balls", "");
    let sample = Sample::<u32>::new("balls", "");
    let metrics = MetricsCollection::default();

    metrics.inc(&counter);
    metrics.record(&sample, 3);
  }

  fn with_store() -> (MetricsCollection, CapturingStore) {
    let metrics = MetricsCollection::default();
    let store = CapturingStore::default();
    metrics.begin_store_task(store.clone());
    (metrics, store)
  }

  /// Keeps every batch written to it
  #[derive(Clone, Default)]
  struct CapturingStore(std::sync::Arc<std::sync::Mutex<Vec<MetricsBatch>>>);

  impl CapturingStore {
    /// Closes the collection, then joins every batch written into one
    async fn everything_written(&self, metrics: &MetricsCollection) -> MetricsBatch {
      metrics.close().await;
      let mut all = metrics.take_batch(); // empty, for its shape
      for batch in self.0.lock().unwrap().drain(..) {
        all.definitions.extend(batch.definitions);
        all.counts.extend(batch.counts);
        all.samples.extend(batch.samples);
        all.games.extend(batch.games);
      }
      all
    }
  }

  impl MetricsStore for CapturingStore {
    fn write(&mut self, batch: &MetricsBatch) -> StoreResult<()> {
      self.0.lock().unwrap().push(batch.clone());
      Ok(())
    }
  }
}
