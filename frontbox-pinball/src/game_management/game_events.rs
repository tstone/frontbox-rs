use frontbox::prelude::Event;

/// Routine run once a game is started, before the first turn: the first player has been added. Its participants are
/// what happens before the first player is up (a game-start sound, a start-of-game display). More players can still be
/// added while it runs. The first turn's `TurnStarting` begins once the last participant completes its step. With no
/// routine defined, the first turn begins right away.
///
/// ```rust,ignore
/// app.routine::<GameStarting>(routine![GameStartPresenter]);
/// ```
///
/// It's started at the root, as no player is up yet. The playfield drivers stay off until `TurnStarting`.
#[derive(serde::Serialize, Event, Clone)]
pub struct GameStarting;

/// Routine run once play is over, before the game ends: after the last turn's `TurnEnding` completes, or when the game
/// is quit. Its participants are what happens before the machine returns to attract mode (match, high-score entry, a
/// game-over display). The game is still started while it runs, so a new one can't be started meanwhile. With no
/// routine defined, the game ends right away.
///
/// The game ends on its `RoutineCompleted`, which the game manager hears first. By the time anyone else hears it, the
/// player groups are gone and `GamePhase` is `None`, so systems that only run outside a game (attract mode, the start
/// button) are active again and hear it, final scores included.
///
/// ```rust,ignore
/// app.routine::<GameEnding>(routine![MatchPresenter, HighScoreEntry]);
/// ```
///
/// It's started at the root, as no player is up. The playfield drivers are turned off on its `RoutineStarted`.
#[derive(serde::Serialize, Event, Clone)]
pub struct GameEnding {
  /// Each player's final score, in player order
  pub scores: Vec<(&'static str, u32)>,
  pub reason: GameEndReason,
}

/// Why a game ended
#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEndReason {
  /// The last turn was played
  Completed,
  /// Ended early, e.g. by the quit game combo
  Quit,
  /// Ended by a slam tilt (`SlamTiltSystem`). Participants may skip, e.g. high-score entry.
  SlamTilted,
}

/// Routine run at the start of each turn, after the player's group is activated and before the ball is served. Its
/// participants are what should happen before the ball comes out (player-up display, skill shot selection, etc.); the
/// ball is ejected once the last one completes its step. With no routine defined, the ball is ejected right away.
///
/// ```rust,ignore
/// app.routine::<TurnStarting>(routine![PlayerUpPresenter]);
/// ```
///
/// Other systems watch the routine's events:
/// - `RoutineStarted<TurnStarting>` -- the turn has started. Modes set up for the player here.
/// - `RoutineCompleted<TurnStarting>` -- the ball is being served.
///
/// A turn played on an extra ball has `TurnStartReason::ExtraBall`, so participants can show "shoot again" instead of
/// the usual player-up display. It has the same `turn` as the turn before it.
#[derive(serde::Serialize, Event, Clone)]
pub struct TurnStarting {
  pub current_player: u8,
  pub turn: u8,
  pub reason: TurnStartReason,
}

#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStartReason {
  /// The next turn in player order (ie. default behavior)
  Ordered,
  /// The same player again, on an extra ball. The turn number doesn't advance.
  ExtraBall,
}

/// Emitted once per turn, when the turn became active: the served ball entered play (`BallEnteredPlay`), or a
/// `ball_in_play_switches` switch was hit, whichever came first. Anything timed from the start of play (ball save,
/// skill shot) starts here.
///
/// Unlike `BallEnteredPlay`, which ball management emits for every ball that goes into play (multiball, ball save
/// relaunches), this is emitted only once per turn.
#[derive(serde::Serialize, Event)]
pub struct TurnActive {
  pub current_player: u8,
  pub turn: u8,
}

impl TurnActive {
  pub fn new(current_player: u8, turn: u8) -> Self {
    Self {
      current_player,
      turn,
    }
  }
}

/// Routine run for each drain during a turn (`BallsExitedPlay`), to decide whether it stands. Its participants are
/// whatever can save the ball (ball save, an outlane save), in the order they get the chance. One that saves it puts
/// the balls back into play and calls `ctx.routines.halt`, and the participants after it don't take a step. If none does,
/// the drain stands; when it was the last ball in play (`in_play == 0`), the turn ends with `TurnEnding`. With no
/// routine defined, every drain stands right away.
///
/// ```rust,ignore
/// app.routine::<BallDraining>(routine![BallSaveSystem, OutlaneSave]);
/// ```
///
/// A participant that only saves the last ball completes its step right away while other balls are still in play.
/// Extra balls aren't participants: they're another turn, played once this one ends (`TurnStartReason::ExtraBall`).
///
/// Anything that should only react to drains that aren't saved (a multiball ending once one ball is left) watches
/// `RoutineCompleted<BallDraining>` rather than `BallsExitedPlay`: `RoutineCompletion::AllSteps` means the drain stands,
/// `Halted` that it was saved. Drains are decided one at a time, in the order they happened.
#[derive(serde::Serialize, Event, Clone)]
pub struct BallDraining {
  pub current_player: u8,
  pub turn: u8,
  /// How many balls drained
  pub count: u8,
  /// Balls left in play after the drain
  pub in_play: u8,
}

/// Routine run once the last ball of a turn has left play, or at once when the player tilts. Its participants are the
/// end-of-turn displays and sequences (bonus, etc.); the turn advances once the last one completes its step. With no
/// routine defined, the turn advances right away.
///
/// ```rust,ignore
/// app.routine::<TurnEnding>(routine![BonusPresenter]);
/// ```
///
/// Other systems watch the routine's events:
/// - `RoutineStarted<TurnEnding>` -- play is over, before any participant runs. Modes clean up here.
/// - `RoutineCompleted<TurnEnding>` -- the turn is over. The next one begins, or the game ends, once no ball is left in
///   play: balls released during the routine, or still rolling down after a tilt, are waited for. Drains meanwhile
///   can't be saved.
///
/// A turn ended by a tilt has `TurnEndReason::Tilted`, so participants can skip (a bonus isn't awarded).
///
/// If the player has an extra ball when it completes, they use it, and the next turn is theirs again
/// (`TurnStartReason::ExtraBall`). That includes one granted while this routine ran, by a bonus award say.
#[derive(serde::Serialize, Event, Clone)]
pub struct TurnEnding {
  pub current_player: u8,
  pub turn: u8,
  pub reason: TurnEndReason,
}

#[derive(serde::Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEndReason {
  /// The last ball drained (ie. default behavior)
  Drained,
  /// The player tilted. The balls may still be on the playfield.
  Tilted,
}

/// Emitted when the current player tilts (`GameManagement::tilt`), just before their turn's `TurnEnding` starts. Points
/// are ignored for the rest of the turn.
#[derive(serde::Serialize, Event)]
pub struct PlayerTilted {
  pub player_index: u8,
}

#[derive(serde::Serialize, Event)]
pub struct PlayerAdded;

/// When a player receives points
#[derive(serde::Serialize, Event)]
pub struct PointsAdded {
  pub player_index: u8,
  pub points_received: u32,
  pub total_points: u32,
}

impl PointsAdded {
  pub fn new(player_index: u8, points_received: u32, total_points: u32) -> Self {
    Self {
      player_index,
      points_received,
      total_points,
    }
  }
}

/// Emitted when the current player is granted an extra ball
#[derive(serde::Serialize, Event)]
pub struct ExtraBallGranted {
  pub player_index: u8,
  /// Extra balls the player now has unused, including this one
  pub count: u8,
}

/// Emitted when an extra ball is used: the player's turn is over, and their next one is about to start
#[derive(serde::Serialize, Event)]
pub struct ExtraBallUsed {
  pub player_index: u8,
  /// Extra balls the player still has unused
  pub remaining: u8,
}
