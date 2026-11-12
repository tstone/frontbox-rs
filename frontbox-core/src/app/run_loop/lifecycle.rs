use super::*;

impl RunLoop {
  pub(super) fn spawn_system(&mut self, system: SystemContainer, group: GroupId) {
    let system_id = system.id();

    let was_spawned = match self.groups.get_mut(&group) {
      Some(parent) => match parent.id_by_type_id(&system.type_id()) {
        Some(existing_id) => {
          log::warn!(
            "Could not spawn system {} because group {} already has one ({}). Only one system of a type may run per group.",
            system.name(),
            group,
            existing_id
          );
          false
        }
        None => {
          parent.insert(system);
          true
        }
      },
      None => {
        log::warn!(
          "Could not spawn system {} because group {} does not exist.",
          system.name(),
          group
        );
        false
      }
    };

    if was_spawned {
      let handle = SystemHandle::new(system_id, group);
      let mut system = self.groups.system(handle).unwrap();
      log::info!(
        target: "frontbox_core::systems::lifecycle",
        "🌐 Spawned system {} ({})", system.name(), system.id());

      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      system.retire_on_panic(&ctx, |system| system.initialize(&ctx));

      self.tracers.send(TraceEvent::SystemSpawned {
        id: system_id,
        name: system.name(),
        group,
      });

      let event = SystemSpawned::new(system_id, group);
      let _ = self.app_sender.send(EmitEvent(None, EventBox::new(event)));
    }
  }

  pub(super) fn replace_system(&mut self, old_handle: SystemHandle, new_system: SystemContainer) -> Vec<SystemHandle> {
    let despawned = self.despawn_system(old_handle);
    self.spawn_system(new_system, old_handle.group);
    despawned
  }

  /// Despawns the system, along with the groups it owns. Returns the handles of every system despawned: the system
  /// itself, then those in its groups.
  pub(super) fn despawn_system(&mut self, handle: SystemHandle) -> Vec<SystemHandle> {
    let Some(cell) = self
      .groups
      .get_mut(&handle.group)
      .and_then(|parent| parent.remove(handle.id))
    else {
      return Vec::new();
    };

    {
      let mut system = cell.borrow_mut();
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      // already removed, so a panic here only needs logging
      system.catch_panic(|system| system.on_despawn(&ctx));

      log::info!(
        target: "frontbox_core::systems::lifecycle",
        "🌐 Despawned system {} ({})", system.name(), system.id());

      self.tracers.send(TraceEvent::SystemDespawned {
        id: handle.id,
        group: handle.group,
      });

      let event = SystemDespawned::new(handle.id, handle.group);
      let _ = self.app_sender.send(EmitEvent(None, EventBox::new(event)));
    }

    for state in clear_states_owned_by(&mut self.groups, handle) {
      self.tracers.send(TraceEvent::StateChange {
        group: handle.group,
        name: state.name,
        value: None,
      });
      if let Some(changed) = state.changed_to_default() {
        let _ = self.app_sender.send(EmitEvent(Some(handle.id), changed));
      }
    }

    let mut despawned = vec![handle];
    let owned: Vec<GroupId> = self
      .groups
      .iter()
      .filter(|(_, group)| group.owner == Some(handle))
      .map(|(id, _)| *id)
      .collect();
    for group in owned {
      despawned.extend(self.despawn_system_group(group));
    }
    despawned
  }

  pub(super) fn spawn_system_group(
    &mut self,
    owner: SystemHandle,
    group: GroupHandle,
    child_systems: Vec<ChildSystemContainer>,
    active: bool,
  ) {
    let id = group.id();
    self.groups.insert(id, SystemGroup::owned_by(group.name(), owner));
    log::info!("🌐 Spawned system group {} ({})", group.name(), id);

    self.tracers.send(TraceEvent::SystemGroupSpawned {
      id,
      name: group.name(),
    });

    for child in child_systems {
      self.spawn_system(child.to_system_container(), id);
    }

    if !active {
      self.deactivate_system_group(id);
    }
  }

  /// Despawns the group and its systems, returning the handles of every system despawned
  pub(super) fn despawn_system_group(&mut self, id: GroupId) -> Vec<SystemHandle> {
    let Some(name) = self.groups.get(&id).map(|group| group.name) else {
      log::warn!("No system group {} found, cannot despawn", id);
      return Vec::new();
    };

    let mut despawned = Vec::new();
    for handle in self.groups.handles_in(id) {
      despawned.extend(self.despawn_system(handle));
    }

    self.tracers.send(TraceEvent::SystemGroupDespawned { id, name });

    let _ = self.groups.shift_remove(&id);
    log::info!("🌐 Despawned system group {} ({})", name, id);
    despawned
  }

  pub(super) fn activate_system_group(&mut self, id: GroupId) {
    let Some(group) = self.groups.get_mut(&id) else {
      log::warn!("No system group {} found, cannot activate", id);
      return;
    };

    if !group.active {
      group.active = true;
    } else {
      return;
    }

    for handle in self.groups.handles_in(id) {
      let mut system = self.groups.system(handle).unwrap();
      if system.is_faulted() {
        continue;
      }
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);

      let reactivated = system.retire_on_panic(&ctx, |system| {
        let active = system.is_active(&ctx);
        if active {
          system.on_reactivate(&ctx);
        }
        active
      });
      if reactivated == Some(true) {
        emit_activity_changed(handle, true, &self.app_sender);
      }
    }

    self.tracers.send(TraceEvent::SystemGroupActiveStateChange { id, active: true });
  }

  pub(super) fn deactivate_system_group(&mut self, id: GroupId) {
    let Some(group) = self.groups.get_mut(&id) else {
      log::warn!("No system group {} found, cannot deactivate", id);
      return;
    };

    if group.active {
      group.active = false;
    } else {
      return;
    }

    for handle in self.groups.handles_in(id) {
      let mut system = self.groups.system(handle).unwrap();
      if system.is_faulted() {
        continue;
      }
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      if system.retire_on_panic(&ctx, |system| system.on_deactivate(&ctx))
        .is_some()
      {
        emit_activity_changed(handle, false, &self.app_sender);
      }
    }

    self.tracers.send(TraceEvent::SystemGroupActiveStateChange { id, active: false });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::run_loop::test_support::*;

  #[test]
  fn despawning_an_owner_despawns_its_groups() {
    let mut h = Harness::new();
    let owner = h.spawn_owner();
    let other = h.spawn_owner();
    let player1 = h.spawn_group(owner, "player1");
    let player2 = h.spawn_group(owner, "player2");
    let others = h.spawn_group(other, "other");
    let members: Vec<SystemHandle> = [player1, player2]
      .into_iter()
      .flat_map(|group| h.groups.handles_in(group))
      .collect();

    let despawned = h.despawn(owner);

    assert!(!h.groups.contains_key(&player1));
    assert!(!h.groups.contains_key(&player2));
    assert!(h.groups.contains_key(&others));
    assert_eq!(despawned, [vec![owner], members].concat());
  }

  #[test]
  fn groups_with_the_same_name_are_separate() {
    let mut h = Harness::new();
    let first_owner = h.spawn_owner();
    let second_owner = h.spawn_owner();
    let first = h.spawn_group(first_owner, "player1");
    let second = h.spawn_group(second_owner, "player1");

    h.despawn(first_owner);

    assert!(!h.groups.contains_key(&first));
    assert_eq!(h.groups[&second].owner, Some(second_owner));
    assert_eq!(h.groups.handles_in(second).len(), 1);
  }

  #[test]
  fn a_group_changing_activity_is_broadcast_for_its_systems() {
    let mut h = Harness::new();
    let owner = h.spawn_owner();
    let group = h.spawn_group(owner, "group");
    let members: Vec<(u64, bool)> = h
      .groups
      .handles_in(group)
      .iter()
      .map(|m| (m.id, false))
      .collect();
    h.activity_events();

    h.deactivate_system_group(group);
    assert_eq!(h.activity_events(), members);

    h.activate_system_group(group);
    let reactivated: Vec<(u64, bool)> = members.iter().map(|(id, _)| (*id, true)).collect();
    assert_eq!(h.activity_events(), reactivated);
  }

  /// Panics on its way out
  struct PanicsOnDespawn;

  impl System for PanicsOnDespawn {
    fn on_despawn(&mut self, _ctx: &SystemContext) {
      panic!("despawned");
    }
  }

  #[test]
  fn a_panic_while_despawning_still_despawns() {
    let mut h = Harness::new();
    let handle = h.spawn(PanicsOnDespawn);

    assert_eq!(h.despawn(handle), vec![handle]);
    assert!(!h.groups.contains_system(handle));
  }
}
