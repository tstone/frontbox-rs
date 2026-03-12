use tokio::sync::mpsc;

use crate::prelude::*;
use crate::systems::{SystemCommand, SystemCommands, SystemContainer};

pub struct PlayerSystem {
  initial_scene: Vec<Box<dyn CloneableSystem>>,
  player_scenes: Vec<Vec<SystemContainer>>,
  // Storage for each player
  player_stores: Vec<Store>,
  player_states: Vec<States>,
  /// Index of the current player
  index: u8,
  system_sender: mpsc::UnboundedSender<SystemCommand>,
  system_receiver: mpsc::UnboundedReceiver<SystemCommand>,
  store_sender: mpsc::UnboundedSender<StoreCommand>,
  store_receiver: mpsc::UnboundedReceiver<StoreCommand>,
}

impl PlayerSystem {
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

    let mut player_states = Vec::new();
    player_states.push(States::new());

    let (system_sender, system_receiver) = mpsc::unbounded_channel::<SystemCommand>();
    let (store_sender, store_receiver) = mpsc::unbounded_channel::<StoreCommand>();

    Box::new(Self {
      initial_scene,
      player_scenes,
      player_stores,
      player_states,
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
    ctx: &Context,
    cmds: &Commands,
    mut f: impl FnMut(&mut Box<dyn System>, &Context, &mut Commands),
  ) {
    if let Some(scene) = self.player_scenes.get_mut(self.index as usize) {
      let ctx = ctx.clone_for_manager(
        &self.player_states[self.index as usize],
        &self.player_stores[self.index as usize],
      );
      let mut cmds = cmds.clone_for_manager(self.system_sender.clone(), self.store_sender.clone());

      for system in scene {
        if system.is_active(&ctx) {
          let mut cmds = cmds.clone_for_system(system.id);
          f(&mut system.inner, &ctx, &mut cmds);
        }
      }

      // process system commands
      let current_systems = self.player_scenes.get_mut(self.index as usize).unwrap();
      while let Ok(cmd) = self.system_receiver.try_recv() {
        SystemCommands::process(cmd, current_systems, &ctx, &mut cmds);
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

impl System for PlayerSystem {
  fn on_startup(&mut self, ctx: &Context, cmds: &mut Commands) {
    // call on_startup for all systems in the initial scene
    self.iterate_current_systems(ctx, cmds, |system, ctx, cmds| {
      system.on_startup(ctx, cmds);
    });
  }

  fn on_shutdown(&mut self, ctx: &Context, cmds: &mut Commands) {
    // call on_shutdown for all systems in the current scene
    self.iterate_current_systems(ctx, cmds, |system, ctx, cmds| {
      system.on_shutdown(ctx, cmds);
    });
  }

  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &Context, cmds: &mut Commands) {
    handle_event!(event, {
      PlayerChanged => |e| { self.index = e.current_player_index; }
      PlayerAdded => |_e| { self.add_player();}
    });

    // Forward event to current player scene
    self.iterate_current_systems(ctx, cmds, |system, ctx, cmds| {
      system.on_event(event, ctx, cmds);
    });
  }

  fn on_tick(&mut self, delta: Duration, ctx: &Context, cmds: &mut Commands) {
    self.iterate_current_systems(ctx, cmds, |system, ctx, cmds| {
      system.on_tick(delta, ctx, cmds);
    });
  }

  fn leds(
    &mut self,
    delta_time: Duration,
    ctx: &Context,
  ) -> std::collections::HashMap<&'static str, LedState> {
    let mut leds = std::collections::HashMap::new();
    if let Some(scene) = self.player_scenes.get_mut(self.index as usize) {
      for system in scene {
        if system.is_active(ctx) {
          let system_leds = system.inner.leds(delta_time, ctx);
          leds.extend(system_leds);
        }
      }
    }
    leds
  }
}
