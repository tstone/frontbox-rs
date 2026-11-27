//! # Leaderboards
//!
//! High scores, and every other record a machine keeps: the top scores under a grand champion, a mode's champion, the
//! fastest completion of something, the most loops in a game. Each is a [`Leaderboard`], and [`LeaderboardsSystem`]
//! (at the root) keeps them all, saved to one file.
//!
//! ### Declaring Leaderboards
//!
//! Leaderboards are declared as statics, like metrics and operator configs. The static is how a board is named
//! everywhere else, so nothing depends on two strings matching.
//!
//! ```rust
//! # use frontbox_pinball::*;
//! # use std::sync::LazyLock;
//! # use std::time::Duration;
//! pub static HIGH_SCORES: LazyLock<Leaderboard<u64>> = LazyLock::new(|| {
//!   Leaderboard::highest("High Scores", 5)
//!     .key("high_scores")
//!     .rank_titles(["Grand Champion"]) // the ranks below it are #1 to #4
//!     .defaults([("AAA", 50_000_000), ("BBB", 40_000_000), ("CCC", 30_000_000)])
//! });
//!
//! pub static LOOP_CHAMPION: LazyLock<Leaderboard<u32>> =
//!   LazyLock::new(|| Leaderboard::highest("Loop Champion", 1).key("loop_champion"));
//!
//! pub static CRYPT_FASTEST: LazyLock<Leaderboard<Duration>> =
//!   LazyLock::new(|| Leaderboard::lowest("Fastest Crypt", 1).key("crypt_fastest"));
//! ```
//!
//! The value's type says what's being compared: `u64` for points, `u32` for a count, `Duration` for a time, or any
//! type implementing [`LeaderboardValue`]. `highest` ranks larger values first, and `lowest` smaller ones.
//!
//! The app lists the boards on the system, in the order they're shown. `final_scores` is a board the system fills in
//! itself, with each player's score when the game ends:
//!
//! ```rust,ignore
//! app.system(
//!   LeaderboardsSystem::new("leaderboards.toml")
//!     .final_scores(&HIGH_SCORES)
//!     .board(&LOOP_CHAMPION)
//!     .board(&CRYPT_FASTEST),
//! );
//! ```
//!
//! ### During a Game
//!
//! The system that knows a value submits it for the player who earned it, whenever it likes. Only a player's best
//! value for a board is kept, so a running count can be submitted each time it goes up.
//!
//! ```rust,ignore
//! ctx.systems.with::<LeaderboardsSystem, _>(|boards| boards.submit(&LOOP_CHAMPION, player, self.loops));
//! ```
//!
//! Nothing is saved yet: submissions are held until the game ends, and dropped when the next one starts.
//!
//! ### At Game End
//!
//! A `GameEnding` participant asks what was earned, with every player's submissions ranked together against the
//! boards as they stand. A player's initials are entered once, and go on every board they made:
//!
//! ```rust,ignore
//! let placements = boards.placements(); // who made which board, and at what rank
//! // ...for each player in it, once their initials are in:
//! boards.enter_initials(player, "TMS");
//! ```
//!
//! ### Storage
//!
//! The file holds only what players have earned, each entry with its initials, value and when it was entered. A
//! board's defaults stay in its declaration, and are ranked alongside the earned entries each time the board is read,
//! so changing a board's defaults or size takes effect without touching the file. An earned entry has to beat a
//! value to rank above it; on a tie the older entry stays ahead.

mod leaderboard;
mod leaderboards_system;

pub use leaderboard::*;
pub use leaderboards_system::*;
