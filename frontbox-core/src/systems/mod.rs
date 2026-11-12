//! # Systems
//!
//! <div class="warning">Stability Level: High</div>
//!
//! The heart of Frontbox is a `System`. Almost everything is a System: game modes, credit modes, sound mixer, even the display. Systems interact with the world through events. Systems are just Rust structs, which can manage their own state and be extended with private functions. They have a handful of callback type methods, including general lifecycle `on_spawn` and `on_despawn` handlers.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! struct ExampleSystem {
//!   private_data: u64,
//! }
//!
//! impl System for ExampleSystem {
//!   fn on_spawn(&mut self, ctx: &SystemContext) {
//!     // <do cool stuff here>
//!   }
//! }
//! ```
//!
//! ## What Systems Do
//!
//! - [Emit events](crate::systems::event)
//! - [Schedule cues](crate::systems::cue)
//! - [Interact with other systems](#services)
//! - [Become inactive](#active)
//! - [React to their own lifecycle](#lifecycle)
//!
//! #### Naming
//!
//! It is a Frontbox preference to add a "System" suffix to all systems.
//!
//! - *GOOD*: `TroughSystem`
//! - *BAD*: `Trough`
//!
//! #### Types of Systems
//!
//! - `System` - System which can be started on boot
//! - `SpawnableSystem` - System which can be dynamically started at runtime. Must be `Send + Sync` compatible
//! - `ChildSystem` - System which can be managed within a group (see "System Groups" below). Must implement `Clone`.
//!
//! ## Lifecycle
//!
//! Systems include four lifecycle handlers:
//!
//! - `on_spawn` - Runs once, always, when the system is started up. This lifecycle hook does NOT respect active state.
//! - `on_deactivate` - Runs once when a system was active, but has lost its active state OR its parent group has been deactivated.
//! - `on_reactivate` - Runs once when a system regains active state. Does not run on_spawn.
//! - `on_despawn` - Runs once, always, when the system is being stopped. This lifecycle hook does NOT respect active state.
//!
//! Each of these is also broadcast to every other system as an event, once the handler has run: `SystemSpawned`,
//! `SystemDeactivated`, `SystemReactivated`, and `SystemDespawned`. Use these to react to *another* system's lifecycle;
//! for example, `LedSystem` stops rendering a system's LED declarations while it's inactive.
//!
//! #### Startup
//!
//! Systems can be given on startup, and will be started automatically, or dynamically spawned at runtime. Likewise, running systems can be despawned or replaced.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # struct ExampleSystem;
//! # impl ExampleSystem { fn new() -> Self { Self } }
//! # impl System for ExampleSystem {}
//! # fn example(ctx: &SystemContext) {
//! // Start a new system
//! ctx.systems.spawn(ExampleSystem::new());
//!
//! // Stop the current system and immediately spawn a replacement
//! ctx.systems.replace_self(ExampleSystem::new());
//!
//! // Just stop the current system
//! ctx.systems.despawn_self();
//! # }
//! ```
//!
//! ### Active
//!
//! By default, all systems spawned are active. Systems can be despawned, which removes them entirely, but sometimes it's necessary to keep a system around, having it automatically become active in certain situations. Frontbox supports this feature by way of the `is_active() -> bool` handler.
//!
//! If `is_active` returns `false`, the framework will by skip all other handlers (the ones starting with `on_*`). Within `is_active`, only read access to `self` and `SystemContext` is provided.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # struct ExampleSystem;
//! # // stand-in for `GamePhase` from frontbox-pinball
//! # #[derive(State, Serialize, Default, Clone, PartialEq)]
//! # enum GamePhase { #[default] None, Starting, Active, Ending }
//! // Example system is only active during a game
//! impl System for ExampleSystem {
//!   fn is_active(&self, ctx: &SystemContext) -> bool {
//!     !ctx.states.is(GamePhase::None)
//!   }
//! }
//! ```
//!
//! Prefer reading a [state](mod@crate::systems::state) in `is_active` over asking another system. A state holds the
//! same value for every system while an event is broadcast. Another system's own fields can change partway through
//! the broadcast, so whether a system hears an event would depend on whether it's handled before or after that change.
//!
//! Systems active state can also be controlled by a [parent group](mod@crate::systems::system_group).
//!
//! ### Services
//!
//! Systems can choose to expose public (`pub`) functions that are accessible to other systems. There are a few rules as to which other systems a system can access:
//!
//! - Systems can interact with any sibling systems
//! - Systems can interact with any root systems
//!
//! There are several ways to access another system, depending on the situation:
//!
//! 1. `ctx.systems.get::<S>` - Returns a mutable reference to `Option<S>`
//! 2. `ctx.systems.expect::<S>` - Returns mutable reference to `S`, also panics if it does not exist
//! 3. `ctx.systems.with::<S, _>` - Runs a closure on `S`, logging a warning if it isn't running
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # struct TroughSystem;
//! # impl TroughSystem { fn eject(&mut self) {} }
//! # impl System for TroughSystem {}
//! # fn example(ctx: &SystemContext) {
//! // safe but verbose
//! if let Some(mut trough) = ctx.systems.get::<TroughSystem>() {
//!   trough.eject()
//! }
//!
//! // unsafe when you absolutely know it is present
//! ctx.systems.expect::<TroughSystem>().eject();
//!
//! // a closure, with a warning if it's missing
//! ctx.systems.with::<TroughSystem, _>(|trough| trough.eject());
//! # }
//! ```
//!
//! #### Service Methods
//!
//! When implementing methods called by another service, do not accept a `SystemContext`, but instead a `ServiceContext`. Using `SystemContext` can incorrectly perform operations as the calling System instead of the service. For convenience, accepting an `impl Into<ServiceContext>` means that a SystemContext could be passed in, but will be resolved to a ServiceContext as it does.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! # struct ExampleSystem { handle: SystemHandle }
//! # impl ExampleSystem {
//! fn service_method(&self, ctx: &ServiceContext) {
//!   let ctx = ctx.for_system(self.handle);
//!   // ...
//! }
//! # }
//! ```
//!
//! A `ServiceContext` does not contain the `SystemHandle` (`self.handle`), so this will need to be captured at some other point, likely on spawn.
//!
//! ```rust
//! # use frontbox_core::prelude::*;
//! pub struct ExampleSystem {
//!   handle: SystemHandle
//! }
//!
//! impl ExampleSystem {
//!   pub fn new() -> Self {
//!     Self { handle: SystemHandle::default() }
//!   }
//! }
//!
//! impl System for ExampleSystem {
//!   fn on_spawn(&mut self, ctx: &SystemContext) {
//!     self.handle = ctx.handle();
//!   }
//! }
//! ```
mod context_namespaces;
mod contextual;
pub mod cue;
mod cue_ext;
pub mod event;
mod group_handle;
pub mod routine;
mod service_context;
pub mod state;
mod system;
mod system_container;
mod system_context;
pub mod system_group;
mod system_handle;
mod test_context;

pub use context_namespaces::*;
pub use contextual::*;
pub use cue::*;
pub use cue_ext::*;
pub use event::*;
pub use group_handle::*;
pub use routine::{RoutineCompleted, RoutineCompletion, RoutineDefinition, RoutineStarted};
pub use service_context::*;
pub use state::{State, StateChanged};
pub use system::*;
pub use system_container::*;
pub use system_context::*;
pub use system_group::*;
pub use system_handle::*;
pub use test_context::*;

use std::any::TypeId;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::Debug;

use indexmap::IndexMap;

pub struct Systems {
  systems: IndexMap<u64, RefCell<SystemContainer>>,
  type_to_id: HashMap<TypeId, u64>,
  id_to_type: HashMap<u64, TypeId>,
}

impl Systems {
  pub fn new() -> Self {
    Self {
      systems: IndexMap::new(),
      type_to_id: HashMap::new(),
      id_to_type: HashMap::new(),
    }
  }

  pub fn ids(&self) -> Vec<&u64> {
    self.systems.keys().collect()
  }

  pub fn names(&self) -> Vec<&'static str> {
    self.systems.values().map(|s| s.borrow().name()).collect()
  }

  pub(crate) fn insert(&mut self, system: impl Into<SystemContainer>) {
    let system = system.into();
    self.type_to_id.insert(system.type_id(), system.id());
    self.id_to_type.insert(system.id(), system.type_id());
    self.systems.insert(system.id(), RefCell::new(system));
  }

  pub(crate) fn remove(&mut self, system_id: u64) -> Option<RefCell<SystemContainer>> {
    let result = self.systems.shift_remove(&system_id);
    if let Some(system_type) = self.id_to_type.remove(&system_id) {
      // Only clear the mapping if it still points at this system, so removing an older system of the same type
      // doesn't orphan a newer one.
      if self.type_to_id.get(&system_type) == Some(&system_id) {
        self.type_to_id.remove(&system_type);
      }
    }
    result
  }

  pub fn get_id<T: System + 'static>(&self) -> Option<u64> {
    let type_id = TypeId::of::<T>();
    self.type_to_id.get(&type_id).copied()
  }

  pub(crate) fn id_by_type_id(&self, type_id: &TypeId) -> Option<u64> {
    self.type_to_id.get(type_id).copied()
  }

  pub fn get_by_id(&'_ self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    self.systems.get(system_id).map(|cell| cell.borrow_mut())
  }

  pub fn get_by_type<T: System + 'static>(&'_ self) -> Option<RefMut<'_, T>> {
    let type_id = TypeId::of::<T>();
    let system_id = self.type_to_id.get(&type_id)?;
    self.systems.get(system_id).map(|cell| {
      RefMut::map(cell.borrow_mut(), |container| {
        container
          .downcast_mut::<T>()
          .expect("type_to_id mapping was incorrect")
      })
    })
  }

  pub fn contains<T: System + 'static>(&self) -> bool {
    let type_id = TypeId::of::<T>();
    self.type_to_id.contains_key(&type_id)
  }

  pub fn contains_id(&self, system_id: &u64) -> bool {
    self.systems.contains_key(system_id)
  }

  pub(crate) fn values(&self) -> impl DoubleEndedIterator<Item = &RefCell<SystemContainer>> {
    self.systems.values()
  }
}

impl Debug for Systems {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Systems")
      .field(
        "systems",
        &self
          .systems
          .iter()
          .map(|(id, cell)| (id, cell.borrow().name().to_string()))
          .collect::<IndexMap<_, _>>(),
      )
      .finish()
  }
}
