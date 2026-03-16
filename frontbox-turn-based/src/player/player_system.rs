use frontbox::prebuilt::{ConsumeCredit, CreditedStart};
use frontbox::prelude::*;
use tokio::sync::mpsc;

use crate::*;

pub struct PlayerSystem {
  initial_scene: Vec<Box<dyn ChildSystem>>,
  player_systems: Vec<Vec<SystemContainer>>,
  system_sender: mpsc::UnboundedSender<SystemMessage>,
  system_receiver: mpsc::UnboundedReceiver<SystemMessage>,
  max_players: u8,
}

impl PlayerSystem {
  pub fn new(max_players: u8, initial_scene: Vec<Box<dyn ChildSystem>>) -> Box<Self> {
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
    })
  }

  fn add_player(&mut self, ctx: &mut Context) {
    let game_state = ctx.expect_mut::<PlayersGameState>();
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

    let current_player = ctx.expect::<PlayersGameState>().current_player();

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
    ctx.get::<PlayersGameState>().is_some()
  }
}

impl System for PlayerSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    let max_players = self.max_players.clone();
    ctx.register_command::<StartGame>(move |_, _, ctx| {
      // check if a game is already running
      if ctx.get::<PlayersGameState>().is_some() {
        return;
      }

      ctx.insert(PlayersGameState::new(max_players));
      ctx.emit(GameStarted);
    });

    ctx.register_command::<EndGame>(move |_, _, ctx| {
      // verify the game is already running
      if ctx.get::<PlayersGameState>().is_none() {
        return;
      }

      ctx.remove::<PlayersGameState>();
      ctx.emit(GameEnded);
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
      // TODO: this needs to unregister all for this system
    });
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(event) = event.downcast::<PlayerTurnBeginning>() {
      let game_state = ctx.expect_mut::<PlayersGameState>();
      game_state.current_player = event.current_player;
      game_state.player_turns[event.current_player as usize] = event.turn;
    } else if let Some(_event) = event.downcast::<CreditedStart>() {
      self.add_player(ctx);
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
    let current_player = ctx.expect::<PlayersGameState>().current_player();
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
