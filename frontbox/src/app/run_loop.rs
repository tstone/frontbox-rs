use itertools::Itertools;
use std::collections::HashMap;

use fast_protocol::EventResponse;
use tokio::sync::mpsc;

use crate::app::command_registry::CommandRegistry;
use crate::machine::event_interrupt_registry::EventInterruptRegistry;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use crate::systems::SystemContainer;
use crate::systems::run_system_timers;

pub struct SystemCollection {
  pub systems: HashMap<u64, SystemContainer>,
  pub groups: HashMap<&'static str, SystemGroup>,
}

pub async fn run(
  mut machine: Machine,
  mut store: Store,
  config: AppConfig,
  mut initial_systems: Vec<Box<dyn System>>,
) {
  let mut interrupt_registry = EventInterruptRegistry::new();
  let mut command_registry = CommandRegistry::new();

  let (app_sender, mut app_receiver) = mpsc::unbounded_channel::<AppMessage>();
  machine.set_app_sender(app_sender.clone());

  let mut systems = SystemCollection {
    systems: HashMap::new(),
    groups: HashMap::new(),
  };

  // initialize systems
  initial_systems.insert(0,MachineBridge::new(machine.machine_sender()));
  for system in initial_systems {
    spawn_system(system, None, &mut systems, &mut store, app_sender.clone());
  }
  // TODO: review how this works and make sure it still makes sense in the new architecture
  run_system_timers(config.system_timer_tick.clone(), app_sender.clone());

  // listen for ctrl-c to trigger shutdown
  let tx = app_sender.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c()
      .await
      .expect("failed to listen for ctrl-c");
    let _ = tx.send(AppMessage::Shutdown);
  });

  log::info!("⟳ Run loop started.");
  loop {
    tokio::select! {
      Some(hardware_event) = machine.read_io() => {
        match hardware_event {
          EventResponse::Switch { switch_id, state } => {
            let mut ctx = Context::new(&mut store, 0, app_sender.clone());
            machine.handle_switch_event(switch_id, state, &mut ctx);
          }
        }
      }

      Some(command) = app_receiver.recv() => {
        match command {
          AppMessage::EmitEvent(event) => {
            emit_event(&*event, &mut systems, &mut store, &app_sender, &interrupt_registry);
          }
          AppMessage::SystemTick => {
            handle_system_tick(&mut systems, &mut store, &mut machine, &config, &app_sender).await;
          }
          AppMessage::RegisterInterrupt(system_id, type_id, priority) => {
            interrupt_registry.register(type_id, system_id, priority);
          }
          AppMessage::UnregisterInterrupt(system_id, type_id) => {
            interrupt_registry.unregister(system_id, type_id);
          }
          AppMessage::RegisterCommand(system_id, type_id) => {
            command_registry.register(type_id, system_id);
          }
          AppMessage::UnregisterCommand(_system_id, type_id) => {
            command_registry.unregister(type_id);
        }
          AppMessage::ExecuteCommand(_system_id, cmd) => {
            execute_command(cmd.as_ref(), &command_registry, &mut systems, &mut store, &app_sender);
          }
          AppMessage::UnregisterAllBySystem(system_id) => {
            unregister_all_by_system(system_id, &mut command_registry, &mut interrupt_registry);
          }
          AppMessage::SwitchStates(switch_states) => {
            let switch_lookup = store.get_mut::<SwitchLookup>().unwrap();
            switch_lookup.update_switch_states(switch_states);
          }
          AppMessage::Shutdown => {
            log::warn!("⏹️ Shutdown command received, shutting down...");
            break;
          }
          AppMessage::SpawnSystem(caller_id, system) => {
            spawn_system(Box::new(system), Some(caller_id), &mut systems, &mut store, app_sender.clone());
          }
          AppMessage::ReplaceSystem(system_id, system) => {
            replace_system(system_id, system, &mut systems, &mut store, app_sender.clone());
          }
          AppMessage::DespawnSystem(system_id) => {
            despawn_system(system_id, &mut systems, &mut store, app_sender.clone(), &mut command_registry, &mut interrupt_registry);
          }
          AppMessage::SpawnSystemGroup(group_name, child_systems, active) => {
            spawn_system_group(group_name, child_systems, active, &mut systems, &mut store, app_sender.clone());
          }
          AppMessage::DespawnSystemGroup(group_name) => {
            despawn_system_group(group_name, &mut systems, &mut store, app_sender.clone(), &mut command_registry, &mut interrupt_registry);
          }
          AppMessage::ActivateSystemGroup(group_name) => {
            if let Some(group) = systems.groups.get_mut(group_name) {
              group.activate();
            } else {
              log::warn!("No system group named '{}' found, cannot activate", group_name);
            }
          }
          AppMessage::DeactivateSystemGroup(group_name) => {
            if let Some(group) = systems.groups.get_mut(group_name) {
              group.deactivate();
            } else {
              log::warn!("No system group named '{}' found, cannot deactivate", group_name);
            }
          }
          AppMessage::ClearTimer(system_id, timer_name) => {
            clear_timer(system_id, timer_name, &mut systems);
          }
          AppMessage::SetTimer(system_id, timer_name, duration, mode) => {
            set_timer(system_id, timer_name, duration, mode, &mut systems);
          }
        }
      }
    }

    machine.process_messages().await;
  }

  // Shutdown sequence
  apply_to_systems(&mut systems, &mut store, &app_sender, |system, ctx| {
    system.on_shutdown(ctx);
  });

  // wait a sec to allow systems to process shutdown event and clear timers, etc.
  tokio::time::sleep(Duration::from_millis(1000)).await;
}

/// Searches for a system by ID in the top-level systems and then within groups
fn find_system(system_id: u64, sc: &mut SystemCollection) -> Option<&mut SystemContainer> {
  // check the top-level first for fast access
  if sc.systems.contains_key(&system_id) {
    return sc.systems.get_mut(&system_id);
  } else {
    // otherwise search groups for child systems
    for group in sc.groups.values_mut() {
      if group.contains_key(&system_id) {
        return group.get_mut(&system_id);
      }
    }
  }

  None
}

/// Apply the given closure to all systems, including those within groups, respecting is_active
fn apply_to_systems<F>(
  sc: &mut SystemCollection,
  store: &mut Store,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  mut handler: F,
) where
  F: FnMut(&mut SystemContainer, &mut Context),
{
  // apply to root systems
  for system in sc.systems.values_mut() {
    let mut ctx = Context::new(store, system.id, app_sender.clone());
    if system.is_active(&ctx) {
      handler(system, &mut ctx);
    }
  }

  // apply to child systems in groups
  for group in sc.groups.values_mut() {
    for system in group.values_mut() {
      let mut ctx = Context::new(store, system.id, app_sender.clone());
      if system.is_active(&ctx) {
        handler(system, &mut ctx);
      }
    }
  }
}

fn emit_event(
  event: &dyn Event,
  systems: &mut SystemCollection,
  store: &mut Store,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &EventInterruptRegistry,
) {
  // first pass the event through the interrupt registry. If any interrupt returns `Halt`, stop processing further.
  let mut ctx_template = Context::new(store, 0, app_sender.clone());
  if let Some(interrupts) = interrupt_registry.get_interrupts_for_event(event.type_id()) {
    // interrupts must be evaluated in order of priority (highest first)
    let prioritized_interrupts = interrupts
      .iter()
      .sorted_by_key(|i| std::cmp::Reverse(i.priority));

    for interrupt in prioritized_interrupts {
      if let Some(system) = find_system(interrupt.system_id, systems) {
        let mut ctx = ctx_template.clone_for_system(interrupt.system_id);
        // interrupts must be on an active system to run
        if system.is_active(&ctx) && system.on_interrupt(event, &mut ctx) == InterruptResult::Halt {
          log::info!(
            "Event of type {:?} was halted by interrupt in system {}",
            event.type_id(),
            interrupt.system_id
          );
          return;
        }
        continue;
      };
    }
  }

  // event is broadcast to systems if no interrupt halted it
  apply_to_systems(systems, store, &app_sender, |system, ctx| {
    system.on_event(event, ctx);
  });
}

async fn handle_system_tick(
  systems: &mut SystemCollection,
  store: &mut Store,
  machine: &mut Machine,
  config: &AppConfig,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
) {
  let tick_duration = config.system_timer_tick;
  apply_to_systems(systems, store, &app_sender, |system, ctx| {
    system.on_tick(tick_duration, ctx);
  });
  let mut ctx = Context::new(store, 0, app_sender.clone());
  machine
    .render_leds(systems, config.system_timer_tick, &mut ctx)
    .await;
}

fn execute_command(
  command: &dyn Command,
  command_registry: &CommandRegistry,
  systems: &mut SystemCollection,
  store: &mut Store,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
) {
  if let Some(system_id) = command_registry.get_system_for_command(command.as_any().type_id()) {
    if let Some(system) = find_system(system_id, systems) {
      let mut ctx = Context::new(store, system_id, app_sender.clone());
      // commands must be executed on an active system
      if system.is_active(&ctx) {
        system.on_command(command, &mut ctx);
      }
    } else {
      log::warn!(
        "Command registry returned system ID {} for command, but no such system exists",
        system_id
      );
    }
  } else {
    log::warn!(
      "No system registered to handle command of type {:?}",
      command.as_any().type_id()
    );
  }
}

fn spawn_system(
  system: Box<dyn System>,
  caller_id: Option<u64>,
  sc: &mut SystemCollection,
  store: &mut Store,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  let mut container: SystemContainer = SystemContainer::new_from_system(system);
  let mut ctx = Context::new(store, container.id, app_sender.clone());
  container.on_startup(&mut ctx);

  // check if the caller is a top-level system or a child
  if let Some(caller_id) = caller_id {
    if sc.systems.contains_key(&caller_id) {
      sc.systems.insert(container.id, container);
    } else {
      // if not search groups and spawn there
      for group in sc.groups.values_mut() {
        if group.contains_key(&caller_id) {
          group.insert(container.id, container);
          return;
        }
      }
    }
    log::warn!(
      "No system found with ID {}, cannot spawn new system as child",
      caller_id
    );
  } else {
    sc.systems.insert(container.id, container);
  }
}

fn replace_system(
  system_id: u64,
  new_system: Box<dyn SpawnableSystem>,
  sc: &mut SystemCollection,
  store: &mut Store,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  // find the system to replace, checking top-level first and then groups
  if sc.systems.contains_key(&system_id) {
    if let Some(mut container) = sc.systems.remove(&system_id) {
      let mut ctx = Context::new(store, container.id, app_sender.clone());
      container.on_shutdown(&mut ctx);
    }
    let mut new_container = SystemContainer::new_from_system(Box::new(new_system));
    let mut ctx = Context::new(store, new_container.id, app_sender.clone());
    new_container.on_startup(&mut ctx);
    sc.systems.insert(new_container.id, new_container);
  } else {
    // if not search groups and replace there
    for group in sc.groups.values_mut() {
      if group.contains_key(&system_id) {
        if let Some(mut container) = group.remove(&system_id) {
          let mut ctx = Context::new(store, container.id, app_sender.clone());
          container.on_shutdown(&mut ctx);
        }
        let mut new_container = SystemContainer::new_from_system(Box::new(new_system));
        let mut ctx = Context::new(store, new_container.id, app_sender.clone());
        new_container.on_startup(&mut ctx);
        group.insert(new_container.id, new_container);
        return;
      }
    }
    log::warn!("No system found with ID {}, cannot replace", system_id);
  }
}

fn despawn_system(
  system_id: u64,
  sc: &mut SystemCollection,
  store: &mut Store,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  command_registry: &mut CommandRegistry,
  interrupt_registry: &mut EventInterruptRegistry,
) {
  // check if the system to despawn is a top-level system or a child
  if sc.systems.contains_key(&system_id) {
    if let Some(mut container) = sc.systems.remove(&system_id) {
      unregister_all_by_system(system_id, command_registry, interrupt_registry);
      let mut ctx = Context::new(store, container.id, app_sender.clone());
      container.on_shutdown(&mut ctx);
    }
  } else {
    // if not search groups and despawn there
    for group in sc.groups.values_mut() {
      if group.contains_key(&system_id) {
        if let Some(mut container) = group.remove(&system_id) {
          unregister_all_by_system(system_id, command_registry, interrupt_registry);
          let mut ctx = Context::new(store, container.id, app_sender.clone());
          container.on_shutdown(&mut ctx);
        }
        return;
      }
    }
    log::warn!("No system found with ID {}, cannot despawn", system_id);
  }
}

fn spawn_system_group(
  group_name: &'static str,
  child_systems: Vec<Box<dyn ChildSystem>>,
  active: bool,
  sc: &mut SystemCollection,
  store: &mut Store,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  if sc.groups.contains_key(group_name) {
    log::warn!("System group '{}' already exists, cannot spawn", group_name);
    return;
  }

  let mut group = SystemGroup::new(child_systems);
  if active {
    group.activate();
  } else {
    group.deactivate();
  }

  let mut ctx = Context::new(store, 0, app_sender.clone());
  group.on_startup(&mut ctx);
  sc.groups.insert(group_name, group);
}

fn despawn_system_group(
  group_name: &'static str,
  sc: &mut SystemCollection,
  store: &mut Store,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  command_registry: &mut CommandRegistry,
  interrupt_registry: &mut EventInterruptRegistry,
) {
  if let Some(mut group) = sc.groups.remove(group_name) {
    for system in group.systems.values() {
      unregister_all_by_system(system.id, command_registry, interrupt_registry);
    }

    let mut ctx = Context::new(store, 0, app_sender.clone());
    group.on_shutdown(&mut ctx);
  } else {
    log::warn!(
      "No system group named '{}' found, cannot despawn",
      group_name
    );
  }
}

fn clear_timer(system_id: u64, timer_name: &'static str, sc: &mut SystemCollection) {
  if let Some(system) = find_system(system_id, sc) {
    system.clear_timer(timer_name);
  } else {
    log::warn!(
      "No system found with ID {}, cannot clear timer '{}'",
      system_id,
      timer_name
    );
  }
}

fn set_timer(
  system_id: u64,
  timer_name: &'static str,
  duration: Duration,
  mode: TimerMode,
  sc: &mut SystemCollection,
) {
  if let Some(system) = find_system(system_id, sc) {
    system.set_timer(timer_name, duration, mode);
  } else {
    log::warn!(
      "No system found with ID {}, cannot set timer '{}'",
      system_id,
      timer_name
    );
  }
}

fn unregister_all_by_system(
  system_id: u64,
  command_registry: &mut CommandRegistry,
  interrupt_registry: &mut EventInterruptRegistry,
) {
  command_registry.unregister_by_system(system_id);
  interrupt_registry.unregister_by_system(system_id);
}
