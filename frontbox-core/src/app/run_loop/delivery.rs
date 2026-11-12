use super::*;

impl RunLoop {
  pub(super) fn emit_event(&self, sender: Option<u64>, event_box: EventBox) {
    if event_box.type_id == SYNCHRONIZE_TYPE_ID {
      self.resync.notify_one();
    }

    // notify any monitoring tracers
    self.tracers.send(TraceEvent::Event {
      type_name: event_box.type_name,
      event: event_box.try_json(),
      sender,
    });

    apply_to_systems(&self.groups, &self.base, &self.app_sender, &self.tracers, |system, ctx| {
      system.on_event(event_box.event.as_ref(), ctx);
    });
  }
}

/// Find and apply the closure to the system. Returns None if not found or inactive.
pub(crate) fn apply_to_system<F, T>(
  handle: SystemHandle,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  mut handler: F,
) -> Option<T>
where
  F: FnMut(&mut SystemContainer, &SystemContext) -> T,
{
  let mut result: Option<T> = None;

  if groups.get(&handle.group).is_some_and(|group| group.active)
    && let Some(mut system) = groups.system(handle)
    && !system.is_faulted()
  {
    let ctx = SystemContext::new(base, handle, groups, app_sender);
    result = system.retire_on_panic(&ctx, |system| {
      if handle_active(system, &ctx, app_sender, tracer_txs) {
        Some(handler(system, &ctx))
      } else {
        log::trace!(target: "frontbox_core::inactive", "System {} is inactive, skipping", system.id());
        None
      }
    })
    .flatten();
  }

  result
}

/// Whether the system exists and is active, in an active group. Like any check of activity, this fires the system's
/// deactivate/reactivate handlers if its activity changed since it was last checked.
pub(crate) fn is_system_active(
  handle: SystemHandle,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> bool {
  apply_to_system(handle, groups, base, app_sender, tracer_txs, |_, _| ()).is_some()
}

/// Apply the given closure to all systems, including those within groups, respecting handle_active
pub(crate) fn apply_to_systems<F>(
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
  mut handler: F,
) where
  F: FnMut(&mut SystemContainer, &SystemContext),
{
  for (key, group) in groups {
    if group.active {
      for cell in group.values() {
        let mut system = cell.borrow_mut();
        if system.is_faulted() {
          continue;
        }
        let handle = SystemHandle::new(system.id(), *key);
        let ctx = SystemContext::new(base, handle, groups, app_sender);
        system.retire_on_panic(&ctx, |system| {
          if handle_active(system, &ctx, app_sender, tracer_txs) {
            handler(system, &ctx);
          } else {
            log::trace!(target: "frontbox_core::inactive", "System {} is inactive, skipping", system.id());
          }
        });
      }
    }
  }
}

/// The first part of `EmitEventOriginFirst`: traces the event, hands it to the origin, and queues the rest of the
/// broadcast behind whatever the origin queued in response
pub(crate) fn emit_event_to_origin(
  origin: SystemHandle,
  event_box: EventBox,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  tracer_txs.send(TraceEvent::Event {
    type_name: event_box.type_name,
    event: event_box.try_json(),
    sender: Some(origin.id),
  });

  apply_to_system(
    origin,
    groups,
    base,
    app_sender,
    tracer_txs,
    |system, ctx| {
      system.on_event(event_box.event.as_ref(), ctx);
    },
  );
  let _ = app_sender.send(AppMessage::EmitEventToOthers(origin, event_box));
}

/// The rest of `EmitEventOriginFirst`: broadcasts to every system but the origin, which already handled the event
pub(crate) fn emit_event_to_others(
  handled_by: SystemHandle,
  event_box: EventBox,
  groups: &Groups,
  base: &AppState,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) {
  apply_to_systems(groups, base, app_sender, tracer_txs, |system, ctx| {
    if ctx.handle() != handled_by {
      system.on_event(event_box.event.as_ref(), ctx);
    }
  });
}

/// Whether the system is active, firing its deactivate/reactivate handler and broadcasting the change if its activity
/// changed since it was last checked
fn handle_active(
  system: &mut SystemContainer,
  ctx: &SystemContext,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
  tracer_txs: &TracerSenders,
) -> bool {
  let check = system.handle_active(ctx, tracer_txs);
  if check.changed {
    emit_activity_changed(ctx.handle(), check.active, app_sender);
  }
  check.active
}

/// Broadcast [`SystemReactivated`] or [`SystemDeactivated`], once the system's own handler has run
pub(super) fn emit_activity_changed(
  handle: SystemHandle,
  active: bool,
  app_sender: &mpsc::UnboundedSender<AppMessage>,
) {
  let event = if active {
    EventBox::new(SystemReactivated::new(handle.id, handle.group))
  } else {
    EventBox::new(SystemDeactivated::new(handle.id, handle.group))
  };
  let _ = app_sender.send(EmitEvent(None, event));
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::run_loop::test_support::*;

  /// Active while its flag is set
  struct Toggled(std::rc::Rc<std::cell::Cell<bool>>);

  impl System for Toggled {
    fn is_active(&self, _ctx: &SystemContext) -> bool {
      self.0.get()
    }
  }

  #[test]
  fn a_system_changing_its_own_activity_is_broadcast() {
    let mut h = Harness::new();
    let active = std::rc::Rc::new(std::cell::Cell::new(true));
    let id = h.spawn(Toggled(active.clone())).id;
    h.activity_events();
    let check = |h: &Harness| apply_to_systems(&h.groups, &h.base, &h.app_sender, &h.tracers, |_, _| ());

    check(&h);
    assert_eq!(h.activity_events(), vec![]);

    active.set(false);
    check(&h);
    check(&h);
    assert_eq!(h.activity_events(), vec![(id, false)]);

    active.set(true);
    check(&h);
    assert_eq!(h.activity_events(), vec![(id, true)]);
  }

  /// Records, on each `Poke`, whether it could look itself up
  struct SelfLookup(std::rc::Rc<std::cell::RefCell<Vec<bool>>>);

  impl System for SelfLookup {
    fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
      if event.is::<Poke>() {
        let found = ctx.systems.get::<SelfLookup>().is_some();
        self.0.borrow_mut().push(found);
      }
    }
  }

  /// Calls into `Callee`, which looks this system up again
  struct Caller;

  impl System for Caller {
    fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
      if event.is::<Poke>() {
        ctx.systems.with::<Callee, _>(|callee| callee.call_back(ctx.into()));
      }
    }
  }

  struct Callee {
    handle: SystemHandle,
    found_caller: std::rc::Rc<std::cell::RefCell<Vec<bool>>>,
  }

  impl Callee {
    fn call_back(&mut self, ctx: &ServiceContext) {
      let ctx = ctx.for_system(self.handle);
      let found = ctx.systems.get::<Caller>().is_some();
      self.found_caller.borrow_mut().push(found);
    }
  }

  impl System for Callee {
    fn on_spawn(&mut self, ctx: &SystemContext) {
      self.handle = ctx.handle();
    }
  }

  #[test]
  fn a_system_looking_itself_up_gets_none() {
    let mut h = Harness::new();
    let found = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    h.spawn(SelfLookup(found.clone()));

    h.poke();

    assert_eq!(*found.borrow(), vec![false]);
  }

  #[test]
  fn a_lookup_that_closes_a_cycle_gets_none() {
    let mut h = Harness::new();
    let found = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    h.spawn(Caller);
    h.spawn(Callee {
      handle: SystemHandle::default(),
      found_caller: found.clone(),
    });

    h.poke();

    assert_eq!(*found.borrow(), vec![false]);
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

  /// Counts the `Poke`s it hears
  struct Counter(std::rc::Rc<std::cell::Cell<u32>>);

  impl System for Counter {
    fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
      if event.is::<Poke>() {
        self.0.set(self.0.get() + 1);
      }
    }
  }

  #[test]
  fn a_panicking_system_is_retired_and_the_rest_carry_on() {
    let mut h = Harness::new();
    let heard = std::rc::Rc::new(std::cell::Cell::new(0));
    let before = h.spawn(Counter(heard.clone()));
    let panicky = h.spawn(Panicky);
    h.despawns_requested();

    h.poke();
    h.poke();

    // systems after it still hear the event, and it's only despawned once
    assert_eq!(heard.get(), 2);
    assert_eq!(h.despawns_requested(), vec![panicky]);
    assert!(h.groups.system(panicky).unwrap().is_faulted());
    assert!(!h.groups.system(before).unwrap().is_faulted());
  }
}
