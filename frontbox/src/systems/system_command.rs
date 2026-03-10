use crate::prelude::*;
use crate::systems::SystemContainer;

pub enum SystemCommand {
  SpawnSystem(Box<dyn System>),
  ReplaceSystem(u64, Box<dyn System>),
  DespawnSystem(u64),
  ClearTimer(u64, &'static str),
  SetTimer(u64, &'static str, Duration, TimerMode),
}

pub struct SystemCommands;

impl SystemCommands {
  pub fn process(
    command: SystemCommand,
    systems: &mut Vec<SystemContainer>,
    ctx: &Context,
    cmds: &mut Commands,
  ) {
    match command {
      SystemCommand::SpawnSystem(system) => {
        Self::spawn_system(system, systems, ctx, cmds);
      }
      SystemCommand::ReplaceSystem(system_id, system) => {
        Self::replace_system(system_id, system, systems, ctx, cmds);
      }
      SystemCommand::DespawnSystem(system_id) => {
        Self::despawn_system(system_id, systems, ctx, cmds);
      }
      SystemCommand::ClearTimer(system_id, timer_name) => {
        Self::clear_timer(system_id, timer_name, systems);
      }
      SystemCommand::SetTimer(system_id, timer_name, duration, mode) => {
        Self::set_timer(system_id, timer_name, duration, mode, systems);
      }
    }
  }

  pub fn spawn_system(
    system: Box<dyn System>,
    systems: &mut Vec<SystemContainer>,
    ctx: &Context,
    cmds: &mut Commands,
  ) {
    let mut container = SystemContainer::new_from_system(system);
    log::debug!("Spawning system with ID {}", container.id);
    let mut cmds = cmds.clone_for_system(container.id);
    container.on_startup(ctx, &mut cmds);
    systems.push(container);
  }

  pub fn replace_system(
    system_id: u64,
    system: Box<dyn System>,
    systems: &mut Vec<SystemContainer>,
    ctx: &Context,
    cmds: &mut Commands,
  ) {
    if let Some(pos) = systems.iter().position(|c| c.id == system_id) {
      log::debug!("Replacing system with ID {}", system_id);
      let mut cmds = cmds.clone_for_system(system_id);
      systems[pos].on_shutdown(ctx, &mut cmds);
      systems[pos] = SystemContainer::new_from_system(system);
    } else {
      log::warn!("No system found with ID {}, cannot replace", system_id);
    }
  }

  pub fn despawn_system(
    system_id: u64,
    systems: &mut Vec<SystemContainer>,
    ctx: &Context,
    cmds: &mut Commands,
  ) {
    if let Some(pos) = systems.iter().position(|c| c.id == system_id) {
      log::debug!("Despawning system with ID {}", system_id);
      let mut cmds = cmds.clone_for_system(system_id);
      systems[pos].on_shutdown(ctx, &mut cmds);
      systems.remove(pos);
    } else {
      log::warn!("No system found with ID {}, cannot despawn", system_id);
    }
  }

  pub fn clear_timer(system_id: u64, timer_name: &'static str, systems: &mut Vec<SystemContainer>) {
    if let Some(container) = systems.iter_mut().find(|c| c.id == system_id) {
      log::debug!("Clearing timer '{}' on system ID {}", timer_name, system_id);
      container.clear_timer(timer_name);
    } else {
      log::warn!(
        "No system found with ID {}, cannot clear timer '{}'",
        system_id,
        timer_name
      );
    }
  }

  pub fn set_timer(
    system_id: u64,
    timer_name: &'static str,
    duration: Duration,
    mode: TimerMode,
    systems: &mut Vec<SystemContainer>,
  ) {
    if let Some(container) = systems.iter_mut().find(|c| c.id == system_id) {
      log::debug!(
        "Setting timer '{}' on system ID {} for {:?} with mode {:?}",
        timer_name,
        system_id,
        duration,
        mode
      );
      container.set_timer(timer_name, duration, mode);
    } else {
      log::warn!(
        "No system found with ID {}, cannot set timer '{}'",
        system_id,
        timer_name
      );
    }
  }
}
