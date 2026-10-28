//! # Metrics
//!
//! Metrics record what happens on the machine over time: how often switches are hit, how long games last, how many
//! balls are saved. They show up in two kinds:
//!
//! - A [`Counter`] counts how many times something happened. It can be hit thousands of times a game.
//! - A [`Sample`] records a value measured once per occurrence, such as a game's duration or a player's final score.
//!   Each value is kept, so averages, minimums and maximums can be taken later. Record one a handful of times per game,
//!   not on every switch hit.
//!
//! Frontbox re-exports this crate as `frontbox::metrics`, and every system reaches the machine's
//! [`MetricsCollection`] as `ctx.metrics`. Tools that only read history, such as the console or an audits CLI, depend
//! on this crate alone.
//!
//! ### Declaring Metrics
//!
//! Metrics are declared as statics, like Frontbox's operator configs.
//!
//! ```rust
//! # use frontbox_metrics::*;
//! # use std::sync::LazyLock;
//! # use std::time::Duration;
//! pub static BALL_SAVES: LazyLock<Counter> = LazyLock::new(|| {
//!   Counter::new(
//!     "Ball Saves", // label
//!     "Drains saved by ball save", // description
//!   )
//!   .key("ball_saves")
//!   .category("Ball")
//! });
//!
//! pub static GAME_DURATION: LazyLock<Sample<Duration>> = LazyLock::new(|| {
//!   Sample::new("Game Duration", "Time from game start to game end").key("game_duration")
//! });
//! ```
//!
//! A metric's key is its label unless set with `.key(...)`. `category` groups metrics for display and for resets.
//!
//! There is no registration step. A metric is known from its first recording. Metrics declared on the same key share
//! their recordings when they're the same kind (and, for samples, the same type). A mismatch panics on first use in
//! debug builds; in release it's logged and the later definition isn't recorded.
//!
//! ### Recording
//!
//! ```rust
//! # use frontbox_metrics::*;
//! # use std::sync::LazyLock;
//! # use std::time::Duration;
//! # static BALL_SAVES: LazyLock<Counter> = LazyLock::new(|| Counter::new("Ball Saves", ""));
//! # static GAME_DURATION: LazyLock<Sample<Duration>> = LazyLock::new(|| Sample::new("Game Duration", ""));
//! # let metrics = MetricsCollection::default(); // a system uses `ctx.metrics`
//! metrics.inc(&BALL_SAVES);
//! metrics.add(&BALL_SAVES, 2);
//! metrics.record(&GAME_DURATION, Duration::from_secs(240));
//! ```
//!
//! A sample can be any integer, `f64` or `Duration`, or any type implementing [`SampleType`].
//!
//! ### Games and Players
//!
//! Recordings are tagged with the game and player in progress, so values can be compared per game or per player.
//! Whatever runs the game says when one begins and ends, and whose turn it is. `frontbox-pinball`'s game management
//! does this already.
//!
//! ```rust
//! # use frontbox_metrics::*;
//! # let metrics = MetricsCollection::default();
//! metrics.begin_game();
//! metrics.set_player(0); // counting from 0
//! // ...
//! metrics.end_game();
//! ```
//!
//! ### Storage
//!
//! Recordings are held in memory and flushed to a [`MetricsDb`], an embedded database in the directory set by
//! Frontbox's `BootConfig::metrics_path` (`metrics.db` in the working directory by default). Nothing on the run loop
//! touches disk; a background task writes each flush as one transaction. Counters are written as how much they went up
//! since the previous flush, so a total is the sum of its rows.
//!
//! A flush happens when a game ends, every 30 seconds, and at shutdown. Call [`flush`](MetricsCollection::flush) at
//! other natural pauses; `frontbox-pinball` flushes at the end of each turn. Without a `metrics_path`, flushed
//! recordings are dropped.
//!
//! ### Reading History
//!
//! [`MetricsDb`] also answers queries, such as [`totals`](MetricsDb::totals). In the running machine, readers share the
//! database the app writes to, and see each flush once it's written. Recordings since the last flush aren't in it yet.

mod counter;
mod metric_definition;
mod metrics_collection;
mod metrics_db;
mod metrics_store;
mod sample;

pub use counter::*;
pub use metric_definition::*;
pub use metrics_collection::*;
pub use metrics_db::*;
pub use metrics_store::*;
pub use sample::*;
