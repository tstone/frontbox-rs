use tokio::sync::mpsc;

use crate::machine::context::StoreContext;
use crate::prelude::*;
use crate::systems::{SystemCommand, SystemCommandProcessor, SystemContainer};

pub struct PlayerSuperSystem {
  // The initial scene to use as the basis for each player/team
  pub(crate) initial_scene: Vec<Box<dyn CloneableSystem>>,
  /// Active stack, one per player
  pub(crate) player_scenes: Vec<Vec<SystemContainer>>,
  // Store for each player
  pub(crate) player_stores: Vec<Store>,
  /// Index of the current player
  pub(crate) index: u8,
  system_sender: mpsc::UnboundedSender<SystemCommand>,
  system_receiver: mpsc::UnboundedReceiver<SystemCommand>,
  store_sender: mpsc::UnboundedSender<StoreCommand>,
  store_receiver: mpsc::UnboundedReceiver<StoreCommand>,
}

impl PlayerSuperSystem {
  pub fn new(initial_scene: Vec<Box<dyn CloneableSystem>>) -> Box<Self> {
    let mut player_scenes = Vec::new();
    let copy: Vec<SystemContainer> = initial_scene
      .iter()
      .map(|system| {
        let cloned: Box<dyn CloneableSystem> = dyn_clone::clone_box(&**system);
        SystemContainer::new(next_listener_id(), Box::new(cloned))
      })
      .collect();
    player_scenes.push(copy);

    let mut player_stores = Vec::new();
    player_stores.push(Store::new());

    let (system_sender, system_receiver) = mpsc::unbounded_channel::<SystemCommand>();
    let (store_sender, store_receiver) = mpsc::unbounded_channel::<StoreCommand>();

    Box::new(Self {
      initial_scene,
      player_scenes,
      player_stores,
      index: 0,
      system_sender,
      system_receiver,
      store_sender,
      store_receiver,
    })
  }

  fn add_player(&mut self) {
    let copy: Vec<SystemContainer> = self
      .initial_scene
      .iter()
      .map(|system| {
        let cloned: Box<dyn CloneableSystem> = dyn_clone::clone_box(&**system);
        SystemContainer::new(next_listener_id(), Box::new(cloned))
      })
      .collect();
    self.player_scenes.push(copy);
    self.player_stores.push(Store::new());
  }

  fn iterate_current_systems(
    &mut self,
    ctx: &mut Context,
    mut f: impl FnMut(&mut Box<dyn System>, &mut Context),
  ) {
    if let Some(scene) = self.player_scenes.get_mut(self.index as usize) {
      let current_store = self.player_stores.get(self.index as usize).unwrap();

      for system in scene {
        if system.is_active() {
          // build a new context with the current player's store
          let store_context = StoreContext::new(self.store_sender.clone(), current_store);
          let mut player_ctx =
            ctx.clone_for_manager(system.id, self.system_sender.clone(), store_context);
          f(&mut system.inner, &mut player_ctx);
        }
      }

      // process system commands
      let current_systems = self.player_scenes.get_mut(self.index as usize).unwrap();
      while let Ok(cmd) = self.system_receiver.try_recv() {
        SystemCommandProcessor::process(cmd, current_systems, ctx);
      }

      // process store commands
      let current_store = self.player_stores.get_mut(self.index as usize).unwrap();
      while let Ok(cmd) = self.store_receiver.try_recv() {
        match cmd {
          StoreCommand::Write(f) => {
            f(current_store);
          }
        }
      }
    }
  }
}

impl System for PlayerSuperSystem {
  fn on_startup(&mut self, ctx: &mut Context) {
    // call on_startup for all systems in the initial scene
    self.iterate_current_systems(ctx, |system, ctx| {
      system.on_startup(ctx);
    });
  }

  fn on_shutdown(&mut self, ctx: &mut Context) {
    // call on_shutdown for all systems in the current scene
    self.iterate_current_systems(ctx, |system, ctx| {
      system.on_shutdown(ctx);
    });
  }

  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &mut Context) {
    handle_event!(event, {
      PlayerChanged => |e| { self.index = e.current_player_index; }
      PlayerAdded => |_e| { self.add_player();}
    });

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

  fn leds(&mut self, delta_time: Duration) -> std::collections::HashMap<&'static str, LedState> {
    let mut leds = std::collections::HashMap::new();
    if let Some(scene) = self.player_scenes.get_mut(self.index as usize) {
      for system in scene {
        let system_leds = system.inner.leds(delta_time);
        leds.extend(system_leds);
      }
    }
    leds
  }
}
