use frontbox::prelude::Event;

#[derive(serde::Serialize, Event)]
pub struct GameStarted;
#[derive(serde::Serialize, Event, Clone)]
pub struct GameEnded {
  pub scores: Vec<(&'static str, u32)>,
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
#[derive(serde::Serialize, Event, Clone)]
pub struct TurnStarting {
  pub current_player: u8,
  pub turn: u8,
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
/// whatever can save the ball (ball save, an outlane save, an extra ball), in the order they get the chance. One that
/// saves it puts the balls back into play and calls `halt_routine`, and the participants after it don't take a step.
/// If none does, the drain stands; when it was the last ball in play (`in_play == 0`), the turn ends with
/// `TurnEnding`. With no routine defined, every drain stands right away.
///
/// ```rust,ignore
/// app.routine::<BallDraining>(routine![BallSaveSystem, OutlaneSave, ExtraBall]);
/// ```
///
/// A participant that only saves the last ball, like an extra ball, completes its step right away while other balls
/// are still in play.
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

/// Routine run once the last ball of a turn has left play. Its participants are the end-of-turn displays and sequences
/// (bonus, shoot again, etc.); the turn advances once the last one completes its step. With no routine defined, the
/// turn advances right away.
///
/// ```rust,ignore
/// app.routine::<TurnEnding>(routine![BonusPresenter, ShootAgainPresenter]);
/// ```
///
/// Other systems watch the routine's events:
/// - `RoutineStarted<TurnEnding>` -- play is over, before any participant runs. Modes clean up here.
/// - `RoutineCompleted<TurnEnding>` -- the turn is over, just before the next one begins or the game ends.
#[derive(serde::Serialize, Event, Clone)]
pub struct TurnEnding {
  pub current_player: u8,
  pub turn: u8,
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
