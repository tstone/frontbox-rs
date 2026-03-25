use itertools::Itertools;
use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::machine::event_interrupt_registry::EventInterruptRegistry;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use crate::systems::SystemContainer;
use crate::systems::spawn_system_tick;

pub struct SystemCollection {
  pub systems: Systems,
  pub groups: HashMap<&'static str, SystemGroup>,
}

pub async fn run(
  mut base: ContextBase,
  initial_systems: Vec<SystemContainer>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  mut app_receiver: mpsc::UnboundedReceiver<AppMessage>,
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
) {
  let mut interrupt_registry = EventInterruptRegistry::new();
  let mut sc = SystemCollection {
    systems: Systems::new(),
    groups: HashMap::new(),
  };

  // initialize systems
  for system in initial_systems {
    spawn_system(system, None, &mut sc, &base, app_sender.clone());
  }
  spawn_system_tick(base.app_config.system_interval.clone(), app_sender.clone());

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
      Some(command) = app_receiver.recv() => {
        match command {
          AppMessage::EmitEvent(event) => {
            emit_event(&*event, &mut sc, &base, &app_sender, &interrupt_registry);
          }
          AppMessage::SystemTick => {
            handle_system_tick(&mut sc, &base, &base.app_config, &app_sender, &machine_sender).await;
          }
          AppMessage::RegisterInterrupt(system_id, type_id, priority) => {
            interrupt_registry.register(type_id, system_id, priority);
          }
          AppMessage::UnregisterInterrupt(system_id, type_id) => {
            interrupt_registry.unregister(system_id, type_id);
          }
          AppMessage::UnregisterAllBySystem(system_id) => {
            unregister_all_by_system(system_id, &mut interrupt_registry);
          }
          AppMessage::SingleSwitchState(id, state) => {
            base.switches.update_switch_state(id, state);
          }
          AppMessage::SwitchStates(switch_states) => {
            base.switches.update_switch_states(switch_states);
          }
          AppMessage::Shutdown => {
            log::warn!("⏹️ Shutdown command received, shutting down...");
            break;
          }
          AppMessage::SpawnSystem(caller_id, system) => {
            spawn_system(system.to_system_container(), Some(caller_id), &mut sc, &base, app_sender.clone());
          }
          AppMessage::ReplaceSystem(system_id, system) => {
            replace_system(system_id, system.to_system_container(), &mut sc, &base, app_sender.clone());
          }
          AppMessage::DespawnSystem(system_id) => {
            despawn_system(system_id, &mut sc, &base, app_sender.clone(), &mut interrupt_registry);
          }
          AppMessage::SpawnSystemGroup(group_name, child_systems, active) => {
            spawn_system_group(group_name, child_systems, active, &mut sc, &base, app_sender.clone());
          }
          AppMessage::DespawnSystemGroup(group_name) => {
            despawn_system_group(group_name, &mut sc, &base, app_sender.clone(), &mut interrupt_registry);
          }
          AppMessage::ActivateSystemGroup(group_name) => {
            activate_system_group(group_name, &mut sc, &base, app_sender.clone());
          }
          AppMessage::DeactivateSystemGroup(group_name) => {
            deactivate_system_group(group_name, &mut sc, &base, app_sender.clone());
          }
          AppMessage::CreateCue(system_id, cue_id, cue, signals) => {
            if let Some(cell) = sc.systems.get_by_id(system_id) {
              let mut system = cell.borrow_mut();
              system.create_cue(cue, cue_id, signals);
            } else {
              log::warn!(
                "No system found with ID {}, cannot create cue {:?}",
                system_id,
                cue
              );
            }
          }
          AppMessage::CancelCue(system_id, cue_id) => {
            if let Some(cell) = sc.systems.get_by_id(system_id) {
              let mut system = cell.borrow_mut();
              system.cancel_cue(cue_id);
            } else {
              log::warn!(
                "No system found with ID {}, cannot cancel cue with ID {}",
                system_id,
                cue_id
              );
            }
          }
        }
      }
    }
  }

  // Shutdown sequence
  apply_to_systems(&mut sc, &base, &app_sender, |system, ctx, systems| {
    system.on_shutdown(ctx, systems);
  });

  // wait a sec to allow systems to process shutdown event and clear timers, etc.
  tokio::time::sleep(Duration::from_millis(1000)).await;
}

/// Apply the given closure to all systems, including those within groups, respecting handle_active
fn apply_to_systems<F>(
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  mut handler: F,
) where
  F: FnMut(&mut SystemContainer, &mut Context, &Systems),
{
  // apply to root systems
  let system_ids = sc.systems.ids().copied().collect_vec();
  for system_id in system_ids {
    if let Some(cell) = sc.systems.lease(system_id) {
      {
        let mut system = cell.borrow_mut();
        let mut ctx = Context::new(base, system.id(), app_sender.clone());
        if system.handle_active(&mut ctx, &sc.systems) {
          handler(&mut system, &mut ctx, &sc.systems);
        } else {
          log::trace!("System {} is inactive, skipping", system.id());
        }
      }
      sc.systems.reinsert(system_id, cell);
    }
  }

  // apply to child systems in groups
  for group in sc.groups.values_mut() {
    for mut system in group.values_mut() {
      let mut ctx = Context::new(base, system.id(), app_sender.clone());
      if system.handle_active(&mut ctx, &sc.systems) {
        handler(&mut system, &mut ctx, &sc.systems);
      } else {
        log::trace!("System {} is inactive, skipping", system.id());
      }
    }
  }
}

fn emit_event(
  event: &dyn Signal,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &EventInterruptRegistry,
) {
  // first pass the event through the interrupt registry. If any interrupt returns `Halt`, stop processing further.
  if let Some(interrupts) = interrupt_registry.get_interrupts_for_event(event.type_id()) {
    // interrupts must be evaluated in order of priority (highest first)
    let prioritized_interrupts = interrupts
      .iter()
      .sorted_by_key(|i| std::cmp::Reverse(i.priority));

    for interrupt in prioritized_interrupts {
      if let Some(cell) = sc.systems.lease(interrupt.system_id) {
        {
          let mut system = cell.borrow_mut();
          let mut ctx = Context::new(base, interrupt.system_id, app_sender.clone());
          // interrupts must be on an active system to run
          if system.handle_active(&mut ctx, &sc.systems)
            && system.on_interrupt(event, &mut ctx) == InterruptResult::Halt
          {
            log::info!(
              "Event of type {:?} was halted by interrupt in system {}",
              event.type_id(),
              interrupt.system_id
            );
            return;
          } else {
            log::trace!(
              "Interrupt in system {} did not halt event of type {:?}",
              interrupt.system_id,
              event.type_id()
            );
          }
        }
        sc.systems.reinsert(interrupt.system_id, cell);
        continue;
      };
    }
  }

  // event is broadcast to systems if no interrupt halted it
  apply_to_systems(sc, base, &app_sender, |system, ctx, systems| {
    system.on_event(event, ctx, systems);
  });
}

async fn handle_system_tick(
  systems: &mut SystemCollection,
  base: &ContextBase,
  config: &AppConfig,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  machine_sender: &mpsc::UnboundedSender<MachineMessage>,
) {
  let tick_duration = config.system_interval;

  apply_to_systems(systems, base, &app_sender, |system, ctx, systems| {
    system.on_tick(tick_duration, ctx, systems);
  });

  render_all_leds(systems, base, tick_duration, app_sender, machine_sender);
}

fn spawn_system(
  mut system: SystemContainer,
  caller_id: Option<u64>,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  let mut ctx = Context::new(base, system.id(), app_sender.clone());
  system.on_startup(&mut ctx, &sc.systems);

  // check if the caller is a top-level system or a child
  if let Some(caller_id) = caller_id {
    if sc.systems.contains_id(caller_id) {
      sc.systems.insert(system);
    } else {
      // if not search groups and spawn there
      for group in sc.groups.values_mut() {
        if group.contains_id(caller_id) {
          group.insert(system);
          return;
        }
      }
    }
    log::warn!(
      "No system found with ID {}, cannot spawn new system as child",
      caller_id
    );
  } else {
    sc.systems.insert(system);
  }
}

fn replace_system(
  system_id: u64,
  mut new_system: SystemContainer,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  // find the system to replace, checking top-level first and then groups
  if sc.systems.contains_id(system_id) {
    if let Some(cell) = sc.systems.remove(system_id) {
      let mut system = cell.borrow_mut();
      let mut ctx = Context::new(base, system.id(), app_sender.clone());
      system.on_shutdown(&mut ctx, &sc.systems);
    }
    let mut ctx = Context::new(base, new_system.id(), app_sender.clone());
    new_system.on_startup(&mut ctx, &sc.systems);
    sc.systems.insert(new_system);
  } else {
    // if not search groups and replace there
    for group in sc.groups.values_mut() {
      if group.contains_id(system_id) {
        if let Some(cell) = group.remove(system_id) {
          let mut system = cell.borrow_mut();
          let mut ctx = Context::new(base, system.id(), app_sender.clone());
          system.on_shutdown(&mut ctx, &sc.systems);
        }
        let mut ctx = Context::new(base, new_system.id(), app_sender.clone());
        new_system.on_startup(&mut ctx, &sc.systems);
        group.insert(new_system);
        return;
      }
    }
    log::warn!("No system found with ID {}, cannot replace", system_id);
  }
}

fn despawn_system(
  system_id: u64,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
) {
  // check if the system to despawn is a top-level system or a child
  if sc.systems.contains_id(system_id) {
    if let Some(container) = sc.systems.remove(system_id) {
      let mut system = container.borrow_mut();
      unregister_all_by_system(system_id, interrupt_registry);
      let mut ctx = Context::new(base, system.id(), app_sender.clone());
      system.on_shutdown(&mut ctx, &sc.systems);
    }
  } else {
    // if not search groups and despawn there
    for group in sc.groups.values_mut() {
      if group.contains_id(system_id) {
        if let Some(container) = group.remove(system_id) {
          let mut system = container.borrow_mut();
          unregister_all_by_system(system_id, interrupt_registry);
          let mut ctx = Context::new(base, system.id(), app_sender.clone());
          system.on_shutdown(&mut ctx, &sc.systems);
        }
        return;
      }
    }
    log::warn!("No system found with ID {}, cannot despawn", system_id);
  }
}

fn spawn_system_group(
  group_name: &'static str,
  child_systems: Vec<ChildSystemContainer>,
  active: bool,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  if sc.groups.contains_key(group_name) {
    log::warn!("System group '{}' already exists, cannot spawn", group_name);
    return;
  }

  let mut ctx_template = Context::new(base, 0, app_sender.clone());
  let mut group = SystemGroup::new(
    child_systems
      .into_iter()
      .map(|c| c.to_system_container())
      .collect(),
  );
  if active {
    group.activate(&mut ctx_template, &sc.systems);
  } else {
    group.deactivate(&mut ctx_template, &sc.systems);
  }

  let mut ctx = Context::new(base, 0, app_sender.clone());
  group.on_startup(&mut ctx, &sc.systems);
  sc.groups.insert(group_name, group);
}

fn despawn_system_group(
  group_name: &'static str,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
) {
  if let Some(mut group) = sc.groups.remove(group_name) {
    for cell in group.systems.values() {
      let system = cell.borrow();
      unregister_all_by_system(system.id(), interrupt_registry);
    }

    let mut ctx = Context::new(base, 0, app_sender.clone());
    group.on_shutdown(&mut ctx, &sc.systems);
  } else {
    log::warn!(
      "No system group named '{}' found, cannot despawn",
      group_name
    );
  }
}

fn activate_system_group(
  group_name: &'static str,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  if let Some(group) = sc.groups.get_mut(group_name) {
    let mut ctx = Context::new(base, 0, app_sender.clone());
    group.activate(&mut ctx, &sc.systems);
  } else {
    log::warn!(
      "No system group named '{}' found, cannot activate",
      group_name
    );
  }
}

fn deactivate_system_group(
  group_name: &'static str,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  if let Some(group) = sc.groups.get_mut(group_name) {
    let mut ctx = Context::new(base, 0, app_sender.clone());
    group.deactivate(&mut ctx, &sc.systems);
  } else {
    log::warn!(
      "No system group named '{}' found, cannot deactivate",
      group_name
    );
  }
}

fn unregister_all_by_system(system_id: u64, interrupt_registry: &mut EventInterruptRegistry) {
  interrupt_registry.unregister_by_system(system_id);
}

// TODO: eliminate this in favor of just invoking a `declare_led` function on an LedSystem
fn render_all_leds(
  sc: &mut SystemCollection,
  base: &ContextBase,
  tick_interval: Duration,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  machine_sender: &mpsc::UnboundedSender<MachineMessage>,
) {
  let system_ids = sc.systems.ids().copied().collect_vec();
  let mut declarations = HashMap::new();

  // gather LED declarations from all active systems and child systems (within)
  for system_id in system_ids {
    if let Some(cell) = sc.systems.lease(system_id) {
      {
        let mut system = cell.borrow_mut();
        let mut ctx = Context::new(base, system.id(), app_sender.clone());
        if system.handle_active(&mut ctx, &sc.systems) {
          declarations.insert(system.id(), system.leds(tick_interval, &ctx, &sc.systems));
        } else {
          log::trace!("System {} is inactive, skipping LED rendering", system.id());
        }
      }
      sc.systems.reinsert(system_id, cell);
    }
  }
  for group in sc.groups.values_mut() {
    for mut system in group.systems.values_mut() {
      let mut ctx = Context::new(base, system.id(), app_sender.clone());
      if system.handle_active(&mut ctx, &sc.systems) {
        declarations.insert(system.id(), system.leds(tick_interval, &ctx, &sc.systems));
      } else {
        log::trace!("System {} is inactive, skipping LED rendering", system.id());
      }
    }
  }

  machine_sender
    .send(MachineMessage::RenderLedDeclarations(declarations))
    .ok();
}
