use super::*;

impl RunLoop {
  /// Spawns the system as a child of `parent`, or at the top level. Returns its handle, or `None` if it was refused: a
  /// parent can't have two children of one type, except [`Group`]s and [`Multi`]s. Their children are spawned with
  /// them. A `Multi`'s children are copies, which can share a type as they're left out of the type table.
  pub(super) fn spawn_system(
    &mut self,
    mut system: SystemContainer,
    parent: Option<SystemHandle>,
  ) -> Option<SystemHandle> {
    let system_id = system.id();
    let group = match parent {
      None => ROOT_GROUP,
      Some(parent) => self.children_group_of(parent)?,
    };
    // systems that only hold others, named by whoever made them
    let holder = if let Some(group) = system.downcast_mut::<Group>() {
      Some((group.name(), group.group_description()))
    } else if let Some(multi) = system.downcast_mut::<Multi>() {
      Some((multi.name(), multi.multi_description()))
    } else {
      None
    };
    if let Some((name, description)) = holder {
      system.set_name(name);
      if let Some(description) = description {
        system.set_description(description);
      }
    }
    let is_holder = holder.is_some();

    let siblings = self.groups.get_mut(&group)?;
    if !is_holder && let Some(existing_id) = siblings.id_by_type_id(&system.type_id()) {
      log::warn!(
        target: "frontbox::systems",
        "Could not spawn system {} because its parent already has one ({}). Only one system of a type may run with the \
         same parent.",
        system.name(),
        existing_id
      );
      return None;
    }
    let parent_active = siblings.is_active();
    siblings.add(system);

    let handle = SystemHandle::new(system_id, group);
    let children = {
      let mut system = self.groups.system(handle).unwrap();
      log::info!(
        target: "frontbox::systems",
        "🌐 Spawned system {} ({})", system.name(), system.id());

      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      system.retire_on_panic(&ctx, |system| system.initialize(&ctx, parent_active));

      self.tracers.send(TraceEvent::SystemSpawned {
        id: system_id,
        name: system.name(),
        parent: parent.map(|parent| parent.id),
        active: system.was_active(),
        group: is_holder,
        description: system.description(),
      });
      let mut children: Vec<SystemContainer> = Vec::new();
      if let Some(group) = system.downcast_mut::<Group>() {
        children.extend(
          group
            .take_children()
            .into_iter()
            .map(|child| child.to_system_container()),
        );
      } else if let Some(multi) = system.downcast_mut::<Multi>() {
        children.extend(
          multi
            .take_copies()
            .into_iter()
            .map(|copy| copy.to_system_container()),
        );
      }
      children
    };

    let event = SystemSpawned::new(system_id);
    let _ = self.app_sender.send(EmitEvent(None, EventBox::new(event)));

    for child in children {
      self.spawn_system(child, Some(handle));
    }
    Some(handle)
  }

  /// The group holding the system's children, made for its first child. `None` if the system isn't running.
  fn children_group_of(&mut self, parent: SystemHandle) -> Option<GroupId> {
    if let Some(existing) = self.groups.children_group(parent) {
      return Some(existing);
    }
    let Some(parent_system) = self.groups.system(parent) else {
      log::warn!(target: "frontbox::systems", "Could not spawn a child of system {}, which isn't running", parent.id);
      return None;
    };
    let active = parent_system.was_active();
    let copies = parent_system.type_id() == TypeId::of::<Multi>();
    drop(parent_system);

    let id = GroupId::next();
    self
      .groups
      .insert(id, SystemGroup::owned_by(parent, active, copies));
    Some(id)
  }

  pub(super) fn replace_system(
    &mut self,
    old_handle: SystemHandle,
    new_system: SystemContainer,
  ) -> Vec<SystemHandle> {
    let parent = self.groups.parent(old_handle);
    let despawned = self.despawn_system(old_handle, DespawnCause::Replaced);
    self.spawn_system(new_system, parent);
    despawned
  }

  /// Despawns the system and its children. Returns the handles of every system despawned: the system itself, then its
  /// children's. A system despawning itself after a panic is traced as `DespawnCause::Panicked`.
  pub(super) fn despawn_system(
    &mut self,
    handle: SystemHandle,
    cause: DespawnCause,
  ) -> Vec<SystemHandle> {
    let Some(cell) = self
      .groups
      .get_mut(&handle.group)
      .and_then(|parent| parent.remove(handle.id))
    else {
      return Vec::new();
    };

    {
      let mut system = cell.borrow_mut();
      let cause = match cause {
        DespawnCause::Itself if system.is_faulted() => DespawnCause::Panicked,
        cause => cause,
      };
      let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
      // already removed, so a panic here only needs logging
      system.catch_panic(|system| system.on_despawn(&ctx));

      log::info!(
        target: "frontbox::systems",
        "🌐 Despawned system {} ({})", system.name(), system.id());

      self.tracers.send(TraceEvent::SystemDespawned {
        id: handle.id,
        cause,
      });

      let event = SystemDespawned::new(handle.id);
      let _ = self.app_sender.send(EmitEvent(None, EventBox::new(event)));
    }

    for state in clear_states_owned_by(&mut self.groups, handle) {
      self.tracers.send(TraceEvent::StateChange {
        name: state.name,
        value: None,
      });
      if let Some(changed) = state.changed_to_default() {
        let _ = self.app_sender.send(EmitEvent(Some(handle.id), changed));
      }
    }

    let mut despawned = vec![handle];
    if let Some(children) = self.groups.children_group(handle) {
      let started_at = match cause {
        DespawnCause::Ancestor { id } => id,
        _ => handle.id,
      };
      for child in self.groups.handles_in(children) {
        despawned.extend(self.despawn_system(child, DespawnCause::Ancestor { id: started_at }));
      }
      let _ = self.groups.shift_remove(&children);
    }
    despawned
  }

  /// The caller's child with this id. Logs a warning and returns `None` for any other system.
  pub(super) fn child_of(
    &self,
    caller: SystemHandle,
    child: u64,
    what: &str,
  ) -> Option<SystemHandle> {
    let found = self
      .groups
      .children_group(caller)
      .map(|group| SystemHandle::new(child, group))
      .filter(|handle| self.groups.contains_system(*handle));
    if found.is_none() {
      log::warn!(
        target: "frontbox::systems",
        "{} tried to {} system {}, which isn't its child, ignored",
        self.groups.system_name(caller).unwrap_or("A despawned system"),
        what,
        child
      );
    }
    found
  }

  /// The parent lets its child run, or stops it. Returns every system whose activity changed: the child and its
  /// children.
  pub(super) fn set_system_enabled(
    &mut self,
    handle: SystemHandle,
    enabled: bool,
  ) -> Vec<SystemHandle> {
    let Some(mut system) = self.groups.system(handle) else {
      return Vec::new();
    };
    system.set_enabled(enabled);
    if system.is_faulted() {
      return Vec::new();
    }
    let group = self.groups.get(&handle.group);
    let parent_active = group.is_some_and(SystemGroup::is_active);
    let cause = ActivityCause::Parent {
      id: group
        .and_then(|group| group.owner)
        .map(|parent| parent.id)
        .unwrap_or_default(),
    };
    let ctx = SystemContext::new(&self.base, handle, &self.groups, &self.app_sender);
    let check = system.retire_on_panic(&ctx, |system| {
      system.check_activity(parent_active, cause, &ctx, &self.tracers)
    });
    drop(system);

    match check {
      Some(check) if check.changed => {
        emit_activity_changed(handle, check.active, &self.app_sender);
        let mut changed = vec![handle];
        changed.extend(cascade_activity(
          handle,
          check.active,
          handle.id,
          &self.groups,
          &self.base,
          &self.app_sender,
          &self.tracers,
        ));
        changed
      }
      _ => Vec::new(),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::cell::Cell;
  use std::rc::Rc;

  use super::*;
  use crate::app::run_loop::test_support::*;

  /// Put the probe, a child of `parent`, through all its handlers: an event, a tick, `parent` deactivating and
  /// reactivating it, and a despawn. Returns what it logged.
  fn run_through_every_handler(
    h: &mut Harness,
    parent: SystemHandle,
    probe: SystemHandle,
    log: &Log<String>,
  ) -> Vec<String> {
    h.emit(Poke);
    h.tick_by(Duration::from_millis(10));
    h.send(AppMessage::DeactivateChild(parent, probe.id));
    h.send(AppMessage::ActivateChild(parent, probe.id));
    h.send(AppMessage::DespawnChild(parent, probe.id));
    h.settle();
    log.take()
  }

  const EVERY_HANDLER: [&str; 7] = [
    "1 on_spawn",
    "1 heard Poke",
    "1 on_tick",
    "1 on_render",
    "1 on_deactivate",
    "1 on_reactivate",
    "1 on_despawn",
  ];

  #[test]
  fn a_system_spawned_by_message_is_a_child_of_the_spawner_and_gets_every_handler() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);

    h.send(AppMessage::SpawnSystem(
      owner,
      Probe::<1>(log.clone()).into(),
    ));
    h.settle();
    let probe = h.children_of(owner)[0];

    assert_eq!(h.groups.parent(probe), Some(owner));
    assert_eq!(
      run_through_every_handler(&mut h, owner, probe, &log),
      EVERY_HANDLER
    );
  }

  #[test]
  fn a_group_member_gets_every_handler() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group_of(owner, "group", vec![Probe::<1>(log.clone()).into()], true);
    let probe = h.children_of(group)[0];

    h.emit(Poke);
    h.tick_by(Duration::from_millis(10));
    // through the group, which its member follows
    h.send(AppMessage::DeactivateChild(owner, group.id));
    h.send(AppMessage::ActivateChild(owner, group.id));
    h.send(AppMessage::DespawnChild(group, probe.id));
    h.settle();

    assert_eq!(log.take(), EVERY_HANDLER);
  }

  #[test]
  fn spawning_is_announced() {
    let mut h = Harness::new();
    let heard = h.listen();

    let probe = h.spawn(Probe::<1>(Log::default()));

    assert_eq!(heard.take(), [Heard::Spawned(probe.id)]);
  }

  #[test]
  fn a_second_system_of_a_type_with_the_same_parent_is_refused() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    h.spawn_under(Some(owner), Probe::<1>(log.clone()));

    h.send(AppMessage::SpawnSystem(
      owner,
      Probe::<1>(log.clone()).into(),
    ));
    h.settle();

    assert_eq!(log.take(), ["1 on_spawn"]);
    assert_eq!(h.children_of(owner).len(), 1);
  }

  #[test]
  fn systems_of_a_type_can_run_under_different_parents() {
    let mut h = Harness::new();
    let log = Log::default();
    let top = h.spawn(Probe::<1>(log.clone()));
    let owner = h.spawn(Owner::<1>);

    h.spawn_under(Some(owner), Probe::<1>(log.clone()));

    assert_eq!(log.take(), ["1 on_spawn", "1 on_spawn"]);
    assert!(h.is_running(top));
  }

  #[test]
  fn several_groups_can_run_with_the_same_parent() {
    let mut h = Harness::new();
    let owner = h.spawn(Owner::<1>);

    h.spawn_group(owner, "player1");
    h.spawn_group(owner, "player2");

    assert_eq!(h.children_of(owner).len(), 2);
  }

  #[test]
  fn a_group_is_named_for_display() {
    let mut h = Harness::new();
    let owner = h.spawn(Owner::<1>);

    let group = h.spawn_group(owner, "player1");

    assert_eq!(h.groups.system_name(group), Some("player1"));
  }

  #[test]
  fn copies_of_a_template_get_their_own_ids() {
    let mut h = Harness::new();
    let owner = h.spawn(Owner::<1>);
    let template: Vec<ChildSystemContainer> = vec![Member.into()];

    let first = h.spawn_group_of(owner, "player1", template.clone(), true);
    let second = h.spawn_group_of(owner, "player2", template, true);

    assert_ne!(h.children_of(first)[0].id, h.children_of(second)[0].id);
  }

  #[test]
  fn a_child_of_a_despawned_system_is_refused() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    h.despawn_system(owner, DespawnCause::Itself);

    h.send(AppMessage::SpawnSystem(
      owner,
      Probe::<1>(log.clone()).into(),
    ));
    h.settle();

    assert_eq!(log.take(), Vec::<String>::new());
  }

  #[test]
  fn despawning_runs_on_despawn_and_is_announced() {
    let mut h = Harness::new();
    let heard = h.listen();
    let log = Log::default();
    let probe = h.spawn(Probe::<1>(log.clone()));
    (heard.take(), log.take());

    h.send(AppMessage::DespawnSystem(probe));
    h.settle();

    assert!(!h.is_running(probe));
    assert_eq!(log.take(), ["1 on_despawn"]);
    assert_eq!(heard.take(), [Heard::Despawned(probe.id)]);
  }

  #[test]
  fn replacing_a_system_keeps_its_parent() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group_of(owner, "group", vec![Probe::<1>(log.clone()).into()], true);
    let old = h.children_of(group)[0];
    log.take();

    h.send(AppMessage::ReplaceSystem(
      old,
      Probe::<2>(log.clone()).into(),
    ));
    h.settle();

    assert_eq!(log.take(), ["1 on_despawn", "2 on_spawn"]);
    assert!(!h.is_running(old));
    assert_eq!(h.children_of(group).len(), 1);
  }

  #[test]
  fn despawning_a_parent_despawns_its_children_and_theirs() {
    let mut h = Harness::new();
    let owner = h.spawn(Owner::<1>);
    let other = h.spawn(Owner::<2>);
    let player1 = h.spawn_group(owner, "player1");
    let player2 = h.spawn_group(owner, "player2");
    let others = h.spawn_group(other, "other");
    let (member1, member2) = (h.children_of(player1)[0], h.children_of(player2)[0]);

    let despawned = h.despawn_system(owner, DespawnCause::Itself);

    assert_eq!(despawned, [owner, player1, member1, player2, member2]);
    assert!(h.is_running(others));
    assert_eq!(h.children_of(others).len(), 1);
  }

  #[test]
  fn only_the_parent_can_change_or_despawn_a_child() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    let other = h.spawn(Owner::<2>);
    let probe = h.spawn_under(Some(owner), Probe::<1>(log.clone()));
    log.take();

    h.send(AppMessage::DeactivateChild(other, probe.id));
    h.send(AppMessage::DespawnChild(other, probe.id));
    h.settle();

    assert_eq!(log.take(), Vec::<String>::new());
    assert!(h.is_running(probe));
  }

  #[test]
  fn a_group_spawned_inactive_deactivates_its_systems() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);

    h.spawn_group_of(owner, "group", vec![Probe::<1>(log.clone()).into()], false);

    assert_eq!(log.take(), ["1 on_spawn", "1 on_deactivate"]);
  }

  #[test]
  fn a_child_spawned_under_an_inactive_parent_starts_inactive() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group_of(owner, "group", vec![], false);

    h.spawn_under(Some(group), Probe::<1>(log.clone()));
    h.emit(Poke);
    assert_eq!(log.take(), ["1 on_spawn"]);

    h.send(AppMessage::ActivateChild(owner, group.id));
    h.settle();
    assert_eq!(log.take(), ["1 on_reactivate"]);
  }

  #[test]
  fn a_change_of_activity_is_broadcast_for_each_descendant() {
    let mut h = Harness::new();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group(owner, "group");
    let member = h.children_of(group)[0];
    let heard = h.listen();

    h.send(AppMessage::DeactivateChild(owner, group.id));
    h.settle();
    assert_eq!(
      heard.take(),
      [Heard::Deactivated(group.id), Heard::Deactivated(member.id)]
    );

    h.send(AppMessage::ActivateChild(owner, group.id));
    h.settle();
    assert_eq!(
      heard.take(),
      [Heard::Reactivated(group.id), Heard::Reactivated(member.id)]
    );
  }

  #[test]
  fn activating_an_active_child_does_nothing() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group_of(owner, "group", vec![Probe::<1>(log.clone()).into()], true);
    log.take();

    h.send(AppMessage::ActivateChild(owner, group.id));
    h.settle();

    assert_eq!(log.take(), Vec::<String>::new());
  }

  #[test]
  fn a_child_deactivated_by_its_parent_stays_inactive_when_an_ancestor_comes_back() {
    let mut h = Harness::new();
    let log = Log::default();
    let owner = h.spawn(Owner::<1>);
    let middle = h.spawn_under(Some(owner), Owner::<2>);
    let probe = h.spawn_under(Some(middle), Probe::<1>(log.clone()));
    log.take();

    h.send(AppMessage::DeactivateChild(middle, probe.id));
    h.send(AppMessage::DeactivateChild(owner, middle.id));
    h.send(AppMessage::ActivateChild(owner, middle.id));
    h.settle();

    assert_eq!(log.take(), ["1 on_deactivate"]);
  }

  /// Active while its flag is set
  struct Toggled(Rc<Cell<bool>>);

  impl System for Toggled {
    fn is_active(&self, _ctx: &SystemContext) -> bool {
      self.0.get()
    }
  }

  #[test]
  fn children_stop_while_their_parent_is_inactive_by_its_own_is_active() {
    let mut h = Harness::new();
    let log = Log::default();
    let active = Rc::new(Cell::new(true));
    let parent = h.spawn(Toggled(active.clone()));
    h.spawn_under(Some(parent), Probe::<1>(log.clone()));
    log.take();

    // noticed the next time the parent is checked, as with any change to `is_active`
    active.set(false);
    h.emit(Poke);
    assert_eq!(log.take(), ["1 on_deactivate"]);

    active.set(true);
    h.emit(Poke);
    assert_eq!(log.take(), ["1 on_reactivate", "1 heard Poke"]);
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

    assert_eq!(h.despawn_system(handle, DespawnCause::Itself), vec![handle]);
    assert!(!h.is_running(handle));
  }
}

#[cfg(test)]
mod lookup_tests {
  use super::*;
  use crate::app::run_loop::test_support::*;

  /// Which system `from` finds when it looks up `T`, by id
  fn found<T: System + 'static>(h: &Harness, from: SystemHandle) -> Option<u64> {
    let ctx = h.service_context().for_system(from);
    ctx.systems.get::<T>().map(|_| ()).and_then(|_| {
      h.groups
        .nearest(from, TypeId::of::<T>())
        .map(|handle| handle.id)
    })
  }

  #[test]
  fn a_system_finds_its_own_children_first() {
    let mut h = Harness::new();
    let parent = h.spawn(Owner::<1>);
    let top = h.spawn(Member);
    let child = h.spawn_under(Some(parent), Member);

    assert_eq!(found::<Member>(&h, parent), Some(child.id));
    assert_ne!(child.id, top.id);
  }

  #[test]
  fn a_system_finds_its_siblings_and_each_level_up() {
    let mut h = Harness::new();
    let top = h.spawn(Owner::<2>);
    let game = h.spawn(Owner::<1>);
    let player = h.spawn_group(game, "player1");
    let member = h.children_of(player)[0];

    // its parent's sibling
    assert_eq!(found::<Owner<2>>(&h, member), Some(top.id));
    // its grandparent, which sits at the top level
    assert_eq!(found::<Owner<1>>(&h, member), Some(game.id));
  }

  #[test]
  fn a_system_doesnt_find_another_systems_children() {
    let mut h = Harness::new();
    let game = h.spawn(Owner::<1>);
    let other = h.spawn(Owner::<2>);
    h.spawn_group(game, "player1");

    assert_eq!(found::<Member>(&h, other), None);
  }

  #[test]
  fn each_group_finds_its_own_copy() {
    let mut h = Harness::new();
    let game = h.spawn(Owner::<1>);
    let template: Vec<ChildSystemContainer> = vec![Member.into(), Owner::<2>.into()];
    let player1 = h.spawn_group_of(game, "player1", template.clone(), true);
    let player2 = h.spawn_group_of(game, "player2", template, true);
    let (member1, owner1) = (h.children_of(player1)[0], h.children_of(player1)[1]);
    let (member2, owner2) = (h.children_of(player2)[0], h.children_of(player2)[1]);

    assert_eq!(found::<Member>(&h, owner1), Some(member1.id));
    assert_eq!(found::<Member>(&h, owner2), Some(member2.id));
  }

  #[test]
  fn get_in_looks_among_a_named_systems_children() {
    let mut h = Harness::new();
    let game = h.spawn(Owner::<1>);
    let other = h.spawn(Owner::<2>);
    let player = h.spawn_group(game, "player1");

    let ctx = h.service_context().for_system(other);
    assert!(ctx.systems.get_in::<Member>(player.id).is_some());
    assert!(
      ctx.systems.get_in::<Member>(game.id).is_none(),
      "only its children, not theirs"
    );
  }
}

#[cfg(test)]
mod cause_tests {
  use std::cell::Cell;
  use std::rc::Rc;

  use tokio::sync::mpsc::UnboundedReceiver;

  use super::*;
  use crate::app::run_loop::test_support::*;

  /// Each activity change traced since the last call, as `(system, active, cause)`
  fn activity(traces: &mut UnboundedReceiver<TraceEvent>) -> Vec<(u64, bool, ActivityCause)> {
    std::iter::from_fn(|| traces.try_recv().ok())
      .filter_map(|trace| match trace {
        TraceEvent::SystemActiveStateChange { id, active, cause } => Some((id, active, cause)),
        _ => None,
      })
      .collect()
  }

  /// Each despawn traced since the last call, as `(system, cause)`
  fn despawns(traces: &mut UnboundedReceiver<TraceEvent>) -> Vec<(u64, DespawnCause)> {
    std::iter::from_fn(|| traces.try_recv().ok())
      .filter_map(|trace| match trace {
        TraceEvent::SystemDespawned { id, cause } => Some((id, cause)),
        _ => None,
      })
      .collect()
  }

  #[test]
  fn a_parent_deactivating_its_child_names_itself_and_the_childs_children_name_the_child() {
    let (mut h, mut traces) = Harness::tracing();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group(owner, "player1");
    let member = h.children_of(group)[0];
    activity(&mut traces);

    h.send(AppMessage::DeactivateChild(owner, group.id));
    h.settle();

    assert_eq!(
      activity(&mut traces),
      [
        (group.id, false, ActivityCause::Parent { id: owner.id }),
        (member.id, false, ActivityCause::Ancestor { id: group.id }),
      ]
    );
  }

  /// Active while its flag is set
  struct Toggled(Rc<Cell<bool>>);

  impl System for Toggled {
    fn is_active(&self, _ctx: &SystemContext) -> bool {
      self.0.get()
    }
  }

  #[test]
  fn a_change_to_its_own_is_active_names_what_was_being_delivered() {
    let (mut h, mut traces) = Harness::tracing();
    let active = Rc::new(Cell::new(true));
    let toggled = h.spawn(Toggled(active.clone()));
    activity(&mut traces);

    active.set(false);
    h.emit(Poke);

    let during = Some(std::any::type_name::<Poke>());
    assert_eq!(
      activity(&mut traces),
      [(toggled.id, false, ActivityCause::IsActive { during })]
    );
  }

  #[test]
  fn a_change_noticed_on_a_tick_says_so() {
    let (mut h, mut traces) = Harness::tracing();
    let active = Rc::new(Cell::new(true));
    let toggled = h.spawn(Toggled(active.clone()));
    activity(&mut traces);

    active.set(false);
    h.tick_by(Duration::from_millis(10));

    assert_eq!(
      activity(&mut traces),
      [(
        toggled.id,
        false,
        ActivityCause::IsActive {
          during: Some("tick")
        }
      )]
    );
  }

  #[test]
  fn despawns_name_who_despawned_them() {
    let (mut h, mut traces) = Harness::tracing();
    let owner = h.spawn(Owner::<1>);
    let group = h.spawn_group(owner, "player1");
    let member = h.children_of(group)[0];
    let other = h.spawn(Owner::<2>);
    despawns(&mut traces);

    h.send(AppMessage::DespawnChild(owner, group.id));
    h.send(AppMessage::DespawnSystem(other));
    h.settle();

    assert_eq!(
      despawns(&mut traces),
      [
        (group.id, DespawnCause::Parent { id: owner.id }),
        (member.id, DespawnCause::Ancestor { id: group.id }),
        (other.id, DespawnCause::Itself),
      ]
    );
  }

  /// Panics on `Poke`
  struct Panicky;

  impl System for Panicky {
    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if event.is::<Poke>() {
        panic!("poked");
      }
    }
  }

  #[test]
  fn a_system_despawned_after_a_panic_says_so() {
    let (mut h, mut traces) = Harness::tracing();
    let panicky = h.spawn(Panicky);
    despawns(&mut traces);

    h.emit(Poke);

    assert_eq!(
      despawns(&mut traces),
      [(panicky.id, DespawnCause::Panicked)]
    );
  }

  /// Each spawn traced since the last call, as `(system, description)`
  fn descriptions(traces: &mut UnboundedReceiver<TraceEvent>) -> Vec<(u64, Option<&'static str>)> {
    std::iter::from_fn(|| traces.try_recv().ok())
      .filter_map(|trace| match trace {
        TraceEvent::SystemSpawned {
          id, description, ..
        } => Some((id, description)),
        _ => None,
      })
      .collect()
  }

  struct Described;

  impl System for Described {
    fn description() -> Option<&'static str> {
      Some("Says what it is for")
    }
  }

  #[test]
  fn a_spawn_carries_the_description_of_its_type_or_of_its_group() {
    let (mut h, mut traces) = Harness::tracing();
    let described = h.spawn(Described);
    let plain = h.spawn(Toggled(Rc::new(Cell::new(true))));
    let group = h.spawn(Group::new("unnamed", vec![]));
    let player = h.spawn(Group::new("player1", vec![]).described("Player 1's systems"));

    assert_eq!(
      descriptions(&mut traces),
      [
        (described.id, Some("Says what it is for")),
        (plain.id, None),
        (group.id, <Group as System>::description()),
        (player.id, Some("Player 1's systems")),
      ]
    );
  }
}

#[cfg(test)]
mod multi_tests {
  use super::*;
  use crate::app::run_loop::test_support::*;

  /// The system `from` finds when it looks up `T`
  fn found<T: System + 'static>(h: &Harness, from: SystemHandle) -> Option<u64> {
    let ctx = h.service_context().for_system(from);
    ctx.systems.get::<T>()?;
    h.groups
      .nearest(from, TypeId::of::<T>())
      .map(|handle| handle.id)
  }

  #[test]
  fn a_multi_runs_each_copy_and_each_hears_events() {
    let mut h = Harness::new();
    let (first, second) = (Log::default(), Log::default());
    let multi = h.spawn(Multi::new(
      "listeners",
      vec![Listener(first.clone()), Listener(second.clone())],
    ));
    first.take();
    second.take();

    h.emit(Poke);

    assert_eq!(h.children_of(multi).len(), 2);
    assert!(first.take().contains(&Heard::Poke));
    assert!(second.take().contains(&Heard::Poke));
  }

  #[test]
  fn copies_cant_be_looked_up_or_join_routines() {
    let mut h = Harness::new();
    let multi = h.spawn(Multi::new("members", vec![Member, Member]));
    let other = h.spawn(Owner::<1>);

    assert_eq!(found::<Member>(&h, other), None);
    assert_eq!(found::<Member>(&h, multi), None);
    // what routines hand off to
    assert!(h.groups.instances(TypeId::of::<Member>()).is_empty());
  }

  #[test]
  fn a_copy_looks_up_systems_above_it() {
    let mut h = Harness::new();
    let top = h.spawn(Owner::<1>);
    let multi = h.spawn(Multi::new("members", vec![Member, Member]));
    let copy = h.children_of(multi)[0];

    assert_eq!(found::<Owner<1>>(&h, copy), Some(top.id));
  }

  #[test]
  fn multis_sit_beside_each_other_and_a_system_of_the_copied_type() {
    let mut h = Harness::new();
    let plain = h.spawn(Member);
    h.spawn(Multi::new("members", vec![Member]));
    h.spawn(Multi::new("more members", vec![Member]));
    let other = h.spawn(Owner::<1>);

    assert_eq!(found::<Member>(&h, other), Some(plain.id));
  }

  #[test]
  fn a_copy_despawning_leaves_the_others() {
    let mut h = Harness::new();
    let multi = h.spawn(Multi::new("members", vec![Member, Member]));
    let (first, second) = (h.children_of(multi)[0], h.children_of(multi)[1]);

    h.despawn_system(first, DespawnCause::Itself);

    assert!(!h.is_running(first));
    assert!(h.is_running(second));
    assert!(h.is_running(multi));
  }
}
