use frontbox::prelude::*;
use tokio::sync::mpsc;

use crate::*;

/// This system provides two main benefits:
///
///   1. Systems are organized by player, so that each player can have their own set of systems that are active only during their turn.
///   2. Player turn management
///
/// ## Outputs
/// - Event: `PlayerTurnBeginning` - Emitted at the start of a player's turn, but before the ball is in play (launched).
/// - Event: `PlayerTurnActive` - Emitted when the ball becomes in play.
/// - Event: `PlayerTurnEnding` - Emitted when the ball goes out of play and is in the trough.
///
/// ## Inputs
/// - Command: `StartGame` - Starts a new game.
/// - Command: `EndGame` - Ends the current game (only valid if a game is started).
/// - Command: `AddPlayer` - Adds a player to the game
/// - Command: `AdvanceTurn` - Advances the turn to the next player. Only processed after a `PlayerTurnEnding` event has been emitted.
/// - Event: `TroughFull` - Used to detect when the ball has gone out of play.
///
/// ## Interrupts
/// - `TroughFull` - Interrupting this event will prevent the player turn from ending. This can be used to implement mechanics like ball saves or extra balls.
///
/// # Arguments:
/// - `max_players` - The maximum number of players allowed in a game
/// - `ball_in_play_switches` - A list of switches that can be used to detect when the ball becomes in play. This could be a plunge lane exit switch, or a list of playfield switches.
///
pub struct IndividualPlayerSystem {
  initial_scene: Vec<Box<dyn ChildSystem>>,
  player_systems: Vec<Vec<SystemContainer>>,
  system_sender: mpsc::UnboundedSender<SystemMessage>,
  system_receiver: mpsc::UnboundedReceiver<SystemMessage>,
  max_players: u8,
  ball_in_play_switches: Vec<&'static str>,
}

impl IndividualPlayerSystem {
  ///
  pub fn new(
    max_players: u8,
    ball_in_play_switches: Vec<&'static str>,
    initial_scene: Vec<Box<dyn ChildSystem>>,
  ) -> Box<Self> {
    let mut player_scenes = Vec::new();
    let copy: Vec<SystemContainer> = initial_scene
      .iter()
      .map(|system| {
        let cloned: Box<dyn ChildSystem> = dyn_clone::clone_box(&**system);
        SystemContainer::new_from_system(Box::new(cloned))
      })
      .collect();
    player_scenes.push(copy);

    let mut player_stores = Vec::new();
    player_stores.push(Store::new());

    let (system_sender, system_receiver) = mpsc::unbounded_channel::<SystemMessage>();

    Box::new(Self {
      initial_scene,
      player_systems: player_scenes,
      system_sender,
      system_receiver,
      max_players,
      ball_in_play_switches,
    })
  }

  fn add_player(&mut self, ctx: &mut Context) {
    let game_state = ctx.expect_mut::<GameState>();
    if game_state.player_count >= game_state.max_players {
      return;
    }

    // create copy of systems for new player
    let copy: Vec<SystemContainer> = self
      .initial_scene
      .iter()
      .map(|system| {
        let cloned: Box<dyn ChildSystem> = dyn_clone::clone_box(&**system);
        SystemContainer::new_from_system(Box::new(cloned))
      })
      .collect();
    self.player_systems.push(copy);

    // increment global state
    game_state.player_count += 1;
    ctx.command(ConsumeCredit);
  }

  fn iterate_current_systems(
    &mut self,
    ctx: &mut Context,
    mut f: impl FnMut(&mut dyn System, &mut Context),
  ) {
    if !self.is_game_started(ctx) {
      return;
    }

    let current_player = ctx.expect::<GameState>().current_player();

    if let Some(child_systems) = self.player_systems.get_mut(current_player as usize) {
      let mut ctx_template = ctx.clone_for_manager(self.system_sender.clone(), 0);

      for system in child_systems {
        let mut ctx = ctx_template.clone_for_system(system.id);
        if system.is_active(&ctx) {
          f(&mut **system, &mut ctx);
        }
      }

      // process system commands
      let current_systems = self
        .player_systems
        .get_mut(current_player as usize)
        .unwrap();
      while let Ok(cmd) = self.system_receiver.try_recv() {
        SystemCommandsProcessor::process(cmd, current_systems, ctx);
      }
    }
  }

  fn is_game_started(&self, ctx: &Context) -> bool {
    ctx.get::<GameState>().is_some()
  }

  fn start_game(ctx: &mut Context, max_players: u8) {
    ctx.insert(GameState::new(max_players));
    ctx.insert(GameStartState::PlayerAddable);
    ctx.emit(GameStarted);
    ctx.emit(PlayerAdded);
    ctx.emit(PlayerTurnBeginning::new(0, 0));
  }

  fn end_game(ctx: &mut Context) {
    // verify the game is already running
    if ctx.get::<GameState>().is_none() {
      return;
    }

    ctx.remove::<GameState>();
    ctx.insert(GameStartState::GameStartable);
    ctx.emit(GameEnded);
  }
}

impl System for IndividualPlayerSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.insert(GameStartState::GameStartable);

    let max_players = self.max_players.clone();
    ctx.register_command::<AddPlayer>(move |_, _, ctx| {
      // check if a game is already running
      if ctx.get::<GameState>().is_some() {
        self.add_player(ctx);
      } else {
        Self::start_game(ctx, max_players);
      }
    });

    // call on_startup for all systems in the initial scene
    self.iterate_current_systems(ctx, |system, ctx| {
      system.on_startup(ctx);
    });
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    // call on_shutdown for all systems in the current scene
    self.iterate_current_systems(ctx, |system, ctx| {
      system.on_shutdown(ctx);
      // TODO: this needs to unregister all for this system -- not available on context
    });
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(game_state) = ctx.get::<GameState>() {
      if let Some(e) = event.downcast::<TroughFull>() {
        if self.ball_in_play_switches.contains(&e.switch.name.as_str()) {
          ctx.emit(PlayerTurnEnding::new(
            game_state.current_player(),
            game_state.turn,
          ));
          ctx.command(AdvanceTurn);
        }
      }
    }

    // Forward event to current player scene
    self.iterate_current_systems(ctx, |system, ctx| {
      system.on_event(event, ctx);
    });
  }

  fn on_tick(&mut self, delta: Duration, ctx: &mut Context) {
    self.iterate_current_systems(ctx, |system, ctx| {
      system.on_tick(delta, ctx);
    });
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    ctx: &Context,
  ) -> std::collections::HashMap<&'static str, LedState> {
    let current_player = ctx.expect::<GameState>().current_player();
    let mut leds = std::collections::HashMap::new();
    if let Some(scene) = self.player_systems.get_mut(current_player as usize) {
      for system in scene {
        if system.is_active(ctx) {
          let system_leds = system.leds(delta_time, ctx);
          leds.extend(system_leds);
        }
      }
    }
    leds
  }
}
