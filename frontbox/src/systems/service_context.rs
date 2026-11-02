use std::any::{TypeId, type_name};
use std::cell::RefMut;
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::app::GroupsExt;
use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;
use crate::systems::routine::RoutineStart;

/// If you're building a service (a System which can be called by other Systems) and you need context, you want to accept _this_ type, then create a system context scoped to your service.
///
/// ```rust
/// # use frontbox::prelude::*;
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
#[derive(Clone)]
pub struct ServiceContext<'a> {
  pub(crate) base: &'a AppState,
  groups: &'a Groups,
  app_sender: mpsc::UnboundedSender<AppMessage>,
}

impl<'a> ServiceContext<'a> {
  pub fn new(
    base: &'a AppState,
    groups: &'a Groups,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> Self {
    Self {
      base,
      groups,
      app_sender,
    }
  }

  /// Dispatch an event to all other active systems
  pub fn emit<E: Event>(&self, sender: u64, event: E) {
    log::debug!(
      "📨 Emitting event {} {:?}",
      type_name::<E>(),
      event.type_id()
    );
    self
      .app_sender
      .send(AppMessage::EmitEvent(Some(sender), EventBox::new(event)))
      .ok();
  }

  // -- event interrupts --

  /// An interrupt is like an event listener but with the ability to halt further processing of the event. Halting an event prevents it from being broadcast.
  /// See [event interrupts](crate::systems::event_interrupts)
  pub fn register_interrupt<E: Event + 'static>(&self, handle: SystemHandle, priority: u16) {
    log::debug!(
      "Registering interrupt for {} by {}",
      type_name::<E>(),
      handle.id,
    );
    self
      .app_sender
      .send(AppMessage::RegisterInterrupt(
        handle,
        TypeId::of::<E>(),
        priority,
      ))
      .ok();
  }

  pub fn unregister_interrupt<E: Event + 'static>(&self, handle: SystemHandle) {
    log::debug!(
      "Unregistering interrupt for {} by {}",
      type_name::<E>(),
      handle.id,
    );
    self
      .app_sender
      .send(AppMessage::UnregisterInterrupt(
        handle.id,
        TypeId::of::<E>(),
      ))
      .ok();
  }

  // -- Systems access --

  /// Lookup a system by type
  pub fn get<T: System + 'static>(&self, handle: SystemHandle) -> Option<RefMut<'_, T>> {
    // Priority is given to nearness: siblings are searched first, then global
    let found = self.groups.nearest(handle.parent_key, TypeId::of::<T>())?;
    self.groups.get(found.parent_key)?.get_by_type::<T>()
  }

  /// Lookup a system by type; panic if does not exist
  pub fn expect<T: System + 'static>(&self, handle: SystemHandle) -> RefMut<'_, T> {
    let system_name = type_name::<T>();
    self.get::<T>(handle).expect(
      format!(
        "Expected system {} was not found. Make sure it was added to the App.",
        system_name
      )
      .as_str(),
    )
  }

  // --- System management ---

  /// Start up a new system
  pub fn spawn_system(
    &self,
    parent_key: &'static str,
    system: impl Into<SpawnableSystemContainer>,
  ) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystem(parent_key, system.into()));
  }

  /// Despawn self and immediately spawn a new system in its place
  pub fn replace_self(&self, handle: SystemHandle, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::ReplaceSystem(handle, system.into()));
  }

  pub fn despawn_self(&self, handle: SystemHandle) {
    let _ = self.app_sender.send(AppMessage::DespawnSystem(handle));
  }

  /// Spawn a group owned by `owner`. Only reachable through `SystemContext`, so the owner is always the caller.
  pub(crate) fn spawn_system_group(
    &self,
    owner: SystemHandle,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) -> GroupHandle {
    let _ = self.app_sender.send(AppMessage::SpawnSystemGroup(
      owner, group_name, systems, active,
    ));
    GroupHandle::new(group_name)
  }

  pub fn despawn_system_group(&self, group: GroupHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystemGroup(group.name()));
  }

  pub fn activate_system_group(&self, group: &GroupHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::ActivateSystemGroup(group.name()));
  }

  pub fn deactivate_system_group(&self, group: &GroupHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::DeactivateSystemGroup(group.name()));
  }

  // --- Cues ---

  pub fn cue(&self, handle: SystemHandle, signal: impl Event + 'static, cue: Cue) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCue(
        handle,
        cue_id,
        cue,
        vec![Box::new(signal)],
      ))
      .ok();
    cue_id
  }

  pub fn cue_cycling(&self, handle: SystemHandle, signals: Vec<Box<dyn Event>>, cue: Cue) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCue(handle, cue_id, cue, signals))
      .ok();
    cue_id
  }

  pub fn cue_timeline(&self, handle: SystemHandle, timeline: CueTimeline) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCueTimeline(handle, cue_id, timeline))
      .ok();
    cue_id
  }

  pub fn cancel_cue(&self, handle: SystemHandle, cue_id: u64) {
    let _ = self.app_sender.send(AppMessage::CancelCue(handle, cue_id));
  }

  // --- Routines ---

  /// Starting and completing need the caller's identity, so they're only public through `SystemContext`
  pub(crate) fn start_routine<E: Event + Clone>(&self, origin: SystemHandle, routine: E) {
    let _ = self.app_sender.send(AppMessage::StartRoutine(RoutineStart {
      origin,
      type_id: TypeId::of::<E>(),
      name: type_name::<E>(),
      routine: Box::new(routine),
      into_started: RoutineStarted::<E>::from_routine,
      into_completed: RoutineCompleted::<E>::from_routine,
    }));
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

  pub fn for_system(&self, handle: SystemHandle) -> SystemContext<'a> {
    SystemContext::new(self.base, handle, self.groups, self.app_sender.clone())
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
