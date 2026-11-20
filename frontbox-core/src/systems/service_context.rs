use std::any::{TypeId, type_name};
use std::cell::RefMut;
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::app::GroupsExt;
use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;
use crate::systems::routine::RoutineBuilder;
use crate::systems::state::{StoredState, state};

/// If you're building a service (a System which can be called by other Systems) and you need context, you want to accept _this_ type, then create a system context scoped to your service.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # struct Wrong { handle: SystemHandle }
/// # impl Wrong {
/// // WRONG:
/// fn service_method(&self, ctx: &SystemContext) {
///   // ...
/// }
/// # }
/// # struct Right { handle: SystemHandle }
/// # impl Right {
///
/// // RIGHT:
/// fn service_method(&self, ctx: &ServiceContext) {
///   let ctx = &ctx.for_system(self.handle);
///   // ...
/// }
/// # }
/// # struct AlsoRight { handle: SystemHandle }
/// # impl AlsoRight {
///
/// // ALSO RIGHT (if you prefer):
/// fn service_method<'a: 'b, 'b>(&self, ctx: impl Into<&'b ServiceContext<'a>>) {
///   let ctx = &ctx.into().for_system(self.handle);
///   // ...
/// }
/// # }
/// ```
#[derive(Clone, Copy)]
pub struct ServiceContext<'a> {
  pub(crate) base: &'a AppState,
  groups: &'a Groups,
  app_sender: &'a mpsc::UnboundedSender<AppMessage>,
  /// Look up drivers, and configure, activate, and deactivate them
  pub drivers: ContextDrivers<'a>,
  /// Look up switches and their state, and configure how they report
  pub switches: ContextSwitches<'a>,
  /// Look up lights, and set LEDs directly
  pub lights: ContextLights<'a>,
}

impl<'a> ServiceContext<'a> {
  pub(crate) fn new(
    base: &'a AppState,
    groups: &'a Groups,
    app_sender: &'a mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      base,
      groups,
      app_sender,
      drivers: ContextDrivers::new(base, groups, app_sender),
      switches: ContextSwitches::new(base, groups, app_sender),
      lights: ContextLights::new(base, groups, app_sender),
    }
  }

  /// Dispatch an event to all other active systems. Takes the sender's identity, so it's only public through
  /// `SystemContext`
  pub(crate) fn emit<E: Event>(&self, sender: u64, event: E) {
    log::debug!(
      target: "frontbox::events",
      "📨 Emitting event {} {:?}",
      type_name::<E>(),
      event.type_id()
    );
    self
      .app_sender
      .send(AppMessage::EmitEvent(Some(sender), EventBox::new(event)))
      .ok();
  }

  // -- Systems access --

  /// Lookup a system by type, the nearest to `handle` first: its children, then its siblings, and so on up to the top
  /// level
  pub(crate) fn get<T: System + 'static>(&self, handle: SystemHandle) -> Option<RefMut<'_, T>> {
    let found = self.groups.nearest(handle, TypeId::of::<T>())?;
    self.borrow_found::<T>(found, handle)
  }

  /// Lookup a system by type among the children of the system with id `parent`
  pub(crate) fn get_in<T: System + 'static>(&self, parent: u64, handle: SystemHandle) -> Option<RefMut<'_, T>> {
    let parent = self.groups.find(parent)?;
    let found = self.groups.child_of_type(parent, TypeId::of::<T>())?;
    self.borrow_found::<T>(found, handle)
  }

  fn borrow_found<T: System + 'static>(&self, found: SystemHandle, caller: SystemHandle) -> Option<RefMut<'_, T>> {
    match self.groups.get(&found.group)?.borrow_by_type::<T>() {
      Ok(system) => system,
      Err(busy) => {
        let caller = self.groups.system_name(caller).unwrap_or("<unknown>");
        log::error!(target: "frontbox::events", "{caller} looked up {}, which is already in use: {busy}", busy.name);
        None
      }
    }
  }

  /// Lookup a system by a trait it provides (see [`System::provides`]): the nearest one matches first
  pub(crate) fn get_dyn<I: ?Sized + 'static>(&self, handle: SystemHandle) -> Option<RefMut<'_, I>> {
    let found = self.groups.nearest(handle, TypeId::of::<I>())?;
    match self.groups.get(&found.group)?.borrow_by_trait::<I>() {
      Ok(system) => system,
      Err(busy) => {
        let caller = self.groups.system_name(handle).unwrap_or("<unknown>");
        log::error!(target: "frontbox::events", "{caller} looked up {}, which is already in use: {busy}", busy.name);
        None
      }
    }
  }

  /// Lookup a system by a trait it provides; panic if none does or it's already in use
  pub(crate) fn expect_dyn<I: ?Sized + 'static>(&self, handle: SystemHandle) -> RefMut<'_, I> {
    self.get_dyn::<I>(handle).unwrap_or_else(|| {
      panic!(
        "Expected a system providing {} was not found, or is already in use (see the error above). Make sure one was \
         added to the App and lists it in `System::provides`.",
        type_name::<I>()
      )
    })
  }

  /// Lookup a system by type; panic if it does not exist or is already in use
  pub(crate) fn expect<T: System + 'static>(&self, handle: SystemHandle) -> RefMut<'_, T> {
    let system_name = type_name::<T>();
    self.get::<T>(handle).expect(
      format!(
        "Expected system {} was not found, or is already in use (see the error above). Make sure it was added to the \
         App.",
        system_name
      )
      .as_str(),
    )
  }

  // --- System management ---

  /// Start up a new system as a child of `parent`, returning its id. Only reachable through `SystemContext`, so the
  /// parent is always the caller.
  pub(crate) fn spawn_child(&self, parent: SystemHandle, system: impl Into<SpawnableSystemContainer>) -> u64 {
    let system = system.into();
    let id = system.id();
    let _ = self.app_sender.send(AppMessage::SpawnSystem(parent, system));
    id
  }

  /// The parent lets its child run, or stops it
  pub(crate) fn set_child_active(&self, parent: SystemHandle, child: u64, active: bool) {
    let message = if active {
      AppMessage::ActivateChild(parent, child)
    } else {
      AppMessage::DeactivateChild(parent, child)
    };
    let _ = self.app_sender.send(message);
  }

  pub(crate) fn despawn_child(&self, parent: SystemHandle, child: u64) {
    let _ = self.app_sender.send(AppMessage::DespawnChild(parent, child));
  }

  /// Despawn self and immediately spawn a new system in its place
  pub(crate) fn replace_self(
    &self,
    handle: SystemHandle,
    system: impl Into<SpawnableSystemContainer>,
  ) {
    let _ = self
      .app_sender
      .send(AppMessage::ReplaceSystem(handle, system.into()));
  }

  pub(crate) fn despawn_self(&self, handle: SystemHandle) {
    let _ = self.app_sender.send(AppMessage::DespawnSystem(handle));
  }


  // --- Timers ---

  pub(crate) fn timer(&self, handle: SystemHandle, signal: impl Event + 'static, timer: Timer) -> u64 {
    let timer_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateTimer(
        handle,
        timer_id,
        timer,
        vec![Box::new(signal)],
      ))
      .ok();
    timer_id
  }

  pub(crate) fn timer_cycling(
    &self,
    handle: SystemHandle,
    signals: Vec<Box<dyn Event>>,
    timer: Timer,
  ) -> u64 {
    let timer_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateTimer(handle, timer_id, timer, signals))
      .ok();
    timer_id
  }

  pub(crate) fn cancel_timer(&self, handle: SystemHandle, timer_id: u64) {
    let _ = self.app_sender.send(AppMessage::CancelTimer(handle, timer_id));
  }

  pub(crate) fn set_timer_paused(&self, handle: SystemHandle, timer_id: u64, paused: bool) {
    let _ = self
      .app_sender
      .send(AppMessage::SetTimerPaused(handle, timer_id, paused));
  }

  // --- Routines ---

  /// Starting, cancelling, completing and halting need the caller's identity, so they're only public through
  /// `SystemContext`
  pub(crate) fn start_routine(&self, routine: RoutineBuilder) -> u64 {
    let routine_id = SystemContainer::next_id();
    let _ = self
      .app_sender
      .send(AppMessage::StartRoutine(routine.into_start(routine_id)));
    routine_id
  }

  pub(crate) fn cancel_routine(&self, origin: SystemHandle, routine_id: u64) {
    let _ = self
      .app_sender
      .send(AppMessage::CancelRoutine(origin, routine_id));
  }

  // --- States ---

  /// The current value of the state `S`, or its default if it hasn't been set. See [states](crate::systems::state)
  pub(crate) fn state<S: State>(&self) -> S {
    state::<S>(self.groups)
  }

  /// Setting needs the caller's identity, since the first to set a state owns it, so it's only public through
  /// `SystemContext`
  pub(crate) fn set_state<S: State>(&self, owner: SystemHandle, value: S) {
    let _ = self
      .app_sender
      .send(AppMessage::SetState(StoredState::new(owner, value)));
  }

  pub(crate) fn report_activity_changed(&self, handle: SystemHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::SystemActivityChanged(handle));
  }

  pub(crate) fn complete_routine_step(&self, handle: SystemHandle, type_id: TypeId) {
    let _ = self
      .app_sender
      .send(AppMessage::CompleteRoutineStep(handle, type_id));
  }

  pub(crate) fn halt_routine(&self, handle: SystemHandle, type_id: TypeId) {
    let _ = self
      .app_sender
      .send(AppMessage::HaltRoutine(handle, type_id));
  }

  pub fn for_system(&self, handle: SystemHandle) -> SystemContext<'a> {
    SystemContext::new(self.base, handle, self.groups, self.app_sender)
  }

  /// Unregisters drivers, clears the EXP network, and halts the framework
  pub fn shutdown(&self, scope: ShutdownScope) {
    let _ = self.app_sender.send(AppMessage::Shutdown(scope));
  }
}

impl Deref for ServiceContext<'_> {
  type Target = AppState;

  fn deref(&self) -> &Self::Target {
    self.base
  }
}
