use std::any::{TypeId, type_name};
use std::cell::RefMut;
use std::ops::Deref;

use tokio::sync::mpsc;

use crate::app::GroupsExt;
use crate::prelude::app_message::{AppMessage, ShutdownScope};
use crate::prelude::*;
use crate::systems::routine::RoutineStart;
use crate::systems::state::{StoredState, nearest_state};

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
  /// Look up LEDs, and set them directly
  pub leds: ContextLeds<'a>,
}

impl<'a> ServiceContext<'a> {
  pub fn new(
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
      leds: ContextLeds::new(base, groups, app_sender),
    }
  }

  /// Dispatch an event to all other active systems. Takes the sender's identity, so it's only public through
  /// `SystemContext`
  pub(crate) fn emit<E: Event>(&self, sender: u64, event: E) {
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

  // -- Systems access --

  /// Lookup a system by type
  pub(crate) fn get<T: System + 'static>(&self, handle: SystemHandle) -> Option<RefMut<'_, T>> {
    // Priority is given to nearness: siblings are searched first, then global
    let found = self.groups.nearest(handle.group, TypeId::of::<T>())?;
    self.groups.get(&found.group)?.get_by_type::<T>()
  }

  /// Lookup a system by type; panic if does not exist
  pub(crate) fn expect<T: System + 'static>(&self, handle: SystemHandle) -> RefMut<'_, T> {
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

  /// Start up a new system in `group`. Systems spawn into their own group through `SystemContext::spawn_system`.
  pub(crate) fn spawn_system(&self, group: GroupId, system: impl Into<SpawnableSystemContainer>) {
    let _ = self
      .app_sender
      .send(AppMessage::SpawnSystem(group, system.into()));
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

  /// Spawn a group owned by `owner`. Only reachable through `SystemContext`, so the owner is always the caller.
  pub(crate) fn spawn_system_group(
    &self,
    owner: SystemHandle,
    group_name: &'static str,
    systems: Vec<ChildSystemContainer>,
    active: bool,
  ) -> GroupHandle {
    let group = GroupHandle::new(GroupId::next(), group_name);
    let _ = self.app_sender.send(AppMessage::SpawnSystemGroup(
      owner,
      group.clone(),
      systems,
      active,
    ));
    group
  }

  pub fn despawn_system_group(&self, group: GroupHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::DespawnSystemGroup(group.id()));
  }

  pub fn activate_system_group(&self, group: &GroupHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::ActivateSystemGroup(group.id()));
  }

  pub fn deactivate_system_group(&self, group: &GroupHandle) {
    let _ = self
      .app_sender
      .send(AppMessage::DeactivateSystemGroup(group.id()));
  }

  // --- Cues ---

  pub(crate) fn cue(&self, handle: SystemHandle, signal: impl Event + 'static, cue: Cue) -> u64 {
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

  pub(crate) fn cue_cycling(
    &self,
    handle: SystemHandle,
    signals: Vec<Box<dyn Event>>,
    cue: Cue,
  ) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCue(handle, cue_id, cue, signals))
      .ok();
    cue_id
  }

  pub(crate) fn cue_timeline(&self, handle: SystemHandle, timeline: CueTimeline) -> u64 {
    let cue_id = SystemContainer::next_id();
    self
      .app_sender
      .send(AppMessage::CreateCueTimeline(handle, cue_id, timeline))
      .ok();
    cue_id
  }

  pub(crate) fn cancel_cue(&self, handle: SystemHandle, cue_id: u64) {
    let _ = self.app_sender.send(AppMessage::CancelCue(handle, cue_id));
  }

  // --- Routines ---

  /// Starting, completing and halting need the caller's identity, so they're only public through `SystemContext`
  /// Participants are looked up from `scope`.
  pub(crate) fn start_routine<E: Event + Clone>(
    &self,
    origin: SystemHandle,
    scope: GroupId,
    routine: E,
  ) {
    let _ = self.app_sender.send(AppMessage::StartRoutine(RoutineStart {
      origin,
      scope,
      type_id: TypeId::of::<E>(),
      name: type_name::<E>(),
      routine: Box::new(routine),
      into_started: RoutineStarted::<E>::from_routine,
      into_completed: RoutineCompleted::<E>::from_routine,
    }));
  }

  // --- States ---

  /// The current value of the state `S`, as seen from `handle`'s group: as set in that group, then the root. The
  /// default if neither has set it. See [states](crate::systems::state)
  pub(crate) fn state<S: State>(&self, handle: SystemHandle) -> S {
    nearest_state::<S>(self.groups, handle.group)
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
