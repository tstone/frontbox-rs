use itertools::Itertools;
use std::cell::RefCell;
use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::app::app_message::AppMessage::EmitEvent;
use crate::app::app_message::EventBox;
use crate::machine::event_interrupt_registry::EventInterruptRegistry;
use crate::prelude::app_message::AppMessage;
use crate::prelude::system_collection::SystemCollection;
use crate::prelude::*;
use crate::systems::SystemContainer;
use crate::systems::spawn_system_tick;

pub async fn run(
  mut base: ContextBase,
  initial_systems: Vec<SystemContainer>,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  mut app_receiver: mpsc::UnboundedReceiver<AppMessage>,
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
  spawn_system_tick(base.system_interval, app_sender.clone());

  // listen for ctrl-c to trigger shutdown
  let tx = app_sender.clone();
  tokio::spawn(async move {
    tokio::signal::ctrl_c()
      .await
      .expect("failed to listen for ctrl-c");
    log::debug!("Ctrl+C signal received.");
    let _ = tx.send(AppMessage::Shutdown);
  });

  log::info!("⟳ Run loop started.");
  loop {
    tokio::select! {
      Some(command) = app_receiver.recv() => {
        log::trace!("AppMessage queue depth: {}", app_receiver.len());
        let start = std::time::Instant::now();

        match command {
          AppMessage::EmitEvent(event_box) => {
            emit_event(event_box, &mut sc, &base, &app_sender, &interrupt_registry);
          }
          AppMessage::SystemTick => {
            handle_system_tick(&mut sc, &base, &app_sender).await;
          }
          AppMessage::RegisterInterrupt(system_id, type_id, priority) => {
            interrupt_registry.register(type_id, system_id, priority);
          }
          AppMessage::UnregisterInterrupt(system_id, type_id) => {
            interrupt_registry.unregister(system_id, type_id);
          }
          AppMessage::UnregisterAllBySystem(system_id) => {
            unregister_all_by_system(&system_id, &mut interrupt_registry);
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
            replace_system(system_id, system.to_system_container(), &mut sc, &base, app_sender.clone(), &mut interrupt_registry);
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
            if let Some(mut system) = sc.get_by_id(&system_id) {
              system.create_cue(cue, cue_id, signals);
            } else {
              log::warn!(
                "No system found with ID {}, cannot create cue {:?}",
                system_id,
                cue
              );
            }
          }
          AppMessage::CreateCueTimeline(system_id, cue_id, timeline) => {
            if let Some(mut system) = sc.get_by_id(&system_id) {
              system.create_cue_timeline(timeline, cue_id);
            } else {
              log::warn!(
                "No system found with ID {}, cannot create cue timeline {:?}",
                system_id,
                cue_id
              );
            }
          }
          AppMessage::CancelCue(system_id, cue_id) => {
            if let Some(mut system) = sc.get_by_id(&system_id) {
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

        log::trace!("Run loop elapsed {}", start.elapsed().as_micros());
      }
    }
  }

  // Shutdown sequence
  apply_to_systems(&mut sc, &base, &app_sender, |system, ctx| {
    system.on_despawn(ctx);
  });

  // wait a sec to allow systems to process shutdown event and clear timers, etc.
  tokio::time::sleep(Duration::from_millis(1000)).await;
}

/// Find and apply the closure to the system. Returns None if not foudn or inactive.
fn apply_to_system<F, T>(
  system_id: u64,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  mut handler: F,
) -> Option<T> where
  F: FnMut(&mut SystemContainer, &mut Context) -> T,
{
  // apply to root systems if present
  if let Some(cell) = sc.systems.lease(system_id) {
    let mut result: Option<T> = None;
    {
      let mut system = cell.borrow_mut();
      let mut ctx = Context::new(base, system.id(), &sc.systems, app_sender.clone());
      if system.handle_active(&ctx) {
        result = Some(handler(&mut system, &mut ctx));
      } else {
        log::trace!("System {} is inactive, skipping", system.id());
      }
    }
    sc.systems.reinsert(system_id, cell);
    result
  } else {
    // otherwise search the groups to apply
    sc.groups.values_mut()
      .find(|g| g.contains_id(&system_id))
      .and_then(|g| g.get_by_id(&system_id))
      .and_then(|mut system| {
        let mut ctx = Context::new(base, system.id(), &sc.systems, app_sender.clone());
        if system.handle_active(&ctx) {
          Some(handler(&mut system, &mut ctx))
        } else {
          log::trace!("System {} is inactive, skipping", system.id());
          None
        }
      })
  }
}

/// Apply the given closure to all systems, including those within groups, respecting handle_active
fn apply_to_systems<F>(
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  mut handler: F,
) where
  F: FnMut(&mut SystemContainer, &mut Context),
{
  // apply to root systems
  let system_ids = sc.systems.ids().copied().collect_vec();
  for system_id in system_ids {
    if let Some(cell) = sc.systems.lease(system_id) {
      {
        let mut system = cell.borrow_mut();
        let mut ctx = Context::new(base, system.id(), &sc.systems, app_sender.clone());
        if system.handle_active(&ctx) {
          handler(&mut system, &mut ctx);
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
      let mut ctx = Context::new(base, system.id(), &sc.systems, app_sender.clone());
      if system.handle_active(&ctx) {
        handler(&mut system, &mut ctx);
      } else {
        log::trace!("System {} is inactive, skipping", system.id());
      }
    }
  }
}

fn emit_event(
  event_box: EventBox,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &EventInterruptRegistry,
) {
  // first pass the event through the interrupt registry. If any interrupt returns `Halt`, stop processing further.
  if let Some(interrupts) = interrupt_registry.get_interrupts_for_event(event_box.type_id) {
    // interrupts must be evaluated in order of priority (highest first)
    let prioritized_interrupts = interrupts
      .iter()
      .sorted_by_key(|i| std::cmp::Reverse(i.priority));

    for interrupt in prioritized_interrupts {
      let interrupt_result = apply_to_system(interrupt.system_id, sc, base, app_sender, |system, ctx| {
        system.on_interrupt(event_box.event.as_ref(), &ctx)
      });

      if interrupt_result == Some(InterruptResult::Halt) {
        log::info!(
          "Event {} was halted by interrupt in system {}",
          event_box.type_name,
          interrupt.system_id
        );
        return;
      }
    }
  } else {
    log::debug!("No interrupts registered for {}", event_box.type_name);
  }

  // event is broadcast to systems if no interrupt halted it
  apply_to_systems(sc, base, app_sender, |system, ctx| {
    system.on_event(event_box.event.as_ref(), ctx);
  });
}

async fn handle_system_tick(
  systems: &mut SystemCollection,
  base: &ContextBase,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
) {
  let tick_duration = base.system_interval;

  // Tick first is where most systems should do any time-based processing
  apply_to_systems(systems, base, app_sender, |system, ctx| {
    system.on_tick(tick_duration, ctx);
  });

  // Render is when systems that depend on what others systems have done (e.g. LED or DMD rendering) occur
  apply_to_systems(systems, base, app_sender, |system, ctx| {
    system.on_render(ctx);
  });
}

fn spawn_system(
  mut system: SystemContainer,
  caller_id: Option<u64>,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
) {
  let system_id = system.id();
  let ctx = Context::new(base, system.id(), &sc.systems, app_sender.clone());
  system.on_spawn(&ctx);

  let parent: &mut Systems = if let Some(caller_id) = caller_id {
    sc.parent(&caller_id).unwrap()
  } else {
    &mut sc.systems
  };

  parent.insert(system);
  let event = SystemSpawned(system_id);
  let _ = app_sender.send(EmitEvent(EventBox::new(event)));
}

fn replace_system(
  system_id: u64,
  new_system: SystemContainer,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
) {
  despawn_system(system_id, sc, base, app_sender.clone(), interrupt_registry);
  spawn_system(new_system, Some(system_id), sc, base, app_sender);
}

/// Despawns the system, returning true if it succeeded
fn despawn_system(
  system_id: u64,
  sc: &mut SystemCollection,
  base: &ContextBase,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  interrupt_registry: &mut EventInterruptRegistry,
) -> bool {
  // check if the system to despawn is a top-level system or a child
  let container: Option<RefCell<SystemContainer>> = if sc.systems.contains_id(&system_id)
    && let Some(container) = sc.systems.remove(system_id)
  {
    Some(container)
  } else {
    // if not search groups and despawn there
    sc.groups
      .values_mut()
      .find(|g| g.contains_id(&system_id))
      .and_then(|group| group.remove(system_id))
  };

  if let Some(container) = container {
    let mut system = container.borrow_mut();
    interrupt_registry.unregister_by_system(&system_id);
    let ctx = Context::new(base, system_id, &sc.systems, app_sender.clone());
    system.on_despawn(&ctx);

    // Emit event that a system was despawned
    let event = SystemDespawned(system_id);
    let _ = app_sender.send(EmitEvent(EventBox::new(event)));
    true
  } else {
    false
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

  let mut ctx_template = Context::new(base, 0, &sc.systems, app_sender.clone());
  let mut group = SystemGroup::new(
    child_systems
      .into_iter()
      .map(|c| c.to_system_container())
      .collect(),
  );
  if active {
    group.activate(&mut ctx_template);
  } else {
    group.deactivate(&mut ctx_template);
  }

  let mut ctx = Context::new(base, 0, &sc.systems, app_sender.clone());
  group.on_spawn(&mut ctx);
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
      unregister_all_by_system(&system.id(), interrupt_registry);
    }

    let mut ctx = Context::new(base, 0, &sc.systems, app_sender.clone());
    group.on_despawn(&mut ctx);
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
    let mut ctx = Context::new(base, 0, &sc.systems, app_sender.clone());
    group.activate(&mut ctx);
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
    let mut ctx = Context::new(base, 0, &sc.systems, app_sender.clone());
    group.deactivate(&mut ctx);
  } else {
    log::warn!(
      "No system group named '{}' found, cannot deactivate",
      group_name
    );
  }
}

fn unregister_all_by_system(system_id: &u64, interrupt_registry: &mut EventInterruptRegistry) {
  interrupt_registry.unregister_by_system(system_id);
}
