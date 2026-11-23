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
//! - [Schedule timers](crate::systems::timer)
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
//! - `ChildSystem` - System which can be copied from a template, such as each player's systems in a [`Group`]. Must
//!   implement `Clone`.
//!
//! ## Lifecycle
//!
//! Systems include four lifecycle handlers:
//!
//! - `on_spawn` - Runs once, always, when the system is started up. This lifecycle hook does NOT respect active state.
//! - `on_deactivate` - Runs once when a system was active, but has lost its active state, been deactivated by its
//!   parent, or one of its ancestors has stopped being active.
//! - `on_reactivate` - Runs once when a system regains active state. Does not run on_spawn.
//! - `on_despawn` - Runs once, always, when the system is being stopped. This lifecycle hook does NOT respect active state.
//!
//! Each of these is also broadcast to every other system as an event, once the handler has run: `SystemSpawned`,
//! `SystemDeactivated`, `SystemReactivated`, and `SystemDespawned`. Use these to react to *another* system's lifecycle;
//! for example, `LightManager` stops rendering a system's light declarations while it's inactive.
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
//! // Start a new system, as a child of this one
//! let child = ctx.systems.spawn(ExampleSystem::new());
//!
//! // Stop the child
//! ctx.systems.despawn(child);
//!
//! // Stop the current system and immediately spawn a replacement
//! ctx.systems.replace_self(ExampleSystem::new());
//!
//! // Just stop the current system
//! ctx.systems.despawn_self();
//! # }
//! ```
//!
//! ## The system tree
//!
//! Systems form a tree. The systems given to the app at startup are the top level, and a system spawned by another is
//! its child. Four rules follow from it:
//!
//! 1. **A system controls only its children.** `ctx.systems.activate`, `deactivate`, and `despawn` take the id
//!    `spawn` returned, and do nothing for any other system. Whatever is done to a system is done to its children too:
//!    despawning it despawns them, and they're active only while it is.
//! 2. **A lookup by type searches the caller's children, then its parent's children, then that system's parent's, up
//!    to the top level.** The first match wins. So a system finds what it spawned, its siblings, and every top-level
//!    system. `ctx.systems.get_in` looks among a named system's children instead.
//! 3. **A type can run once among one parent's children.** A second is refused. [`Group`] is an exception, so a
//!    system can spawn one per player, each holding that player's copy of the same systems. To run several copies of
//!    one type side by side, put them in a [`Multi`]. Lookups and routines don't find a `Multi`'s copies, so there's
//!    never a question of which one was meant.
//! 4. **Events, routines, and states don't depend on where a system is.** Every event reaches every active system, a
//!    parent before its children. A routine is handed to every active system of each type it lists. A state has one
//!    value for the whole machine.
//!
//! ```text
//! BallManager ── TroughSystem
//! Game ──────┬── Group "player1" ── ModeSystem
//!            └── Group "player2" ── ModeSystem
//! ```
//!
//! Here, player 1's `ModeSystem` asks for `TroughSystem` and finds none: it's the ball manager's child, which only
//! the ball manager and its other children see. Deactivating `Group "player1"` stops player 1's `ModeSystem`, and a
//! routine listing `ModeSystem` is handed only to player 2's.
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
//! A system is also inactive while its parent has deactivated it, or while any of its ancestors is inactive (see
//! [the system tree](#the-system-tree)).
//!
//! ### Services
//!
//! Systems can choose to expose public (`pub`) functions that are accessible to other systems.
//!
//! #### Lookups
//!
//! A lookup searches the caller's children, then its siblings, then its parent's siblings, and so on up to the top
//! level, and the first match wins (see [the system tree](#the-system-tree)). There are several ways to access
//! another system, depending on the situation:
//!
//! 1. `ctx.systems.get::<S>` - Returns a mutable reference to `Option<S>`
//! 2. `ctx.systems.expect::<S>` - Returns mutable reference to `S`, also panics if it does not exist
//! 3. `ctx.systems.with::<S, _>` - Runs a closure on `S`, logging a warning if it isn't running
//!
//! Each has a `_dyn` form (`get_dyn`, `expect_dyn`, `with_dyn`) that looks a system up by a trait it provides
//! instead, such as `ctx.systems.get_dyn::<dyn BallManager>()`. See [`System::provides`].
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
pub mod event;
mod group;
mod multi;
mod provides;
pub mod routine;
mod service_context;
pub mod state;
mod status_reporter;
mod system;
mod system_container;
mod system_context;
mod system_group;
mod system_handle;
#[cfg(any(test, feature = "testing"))]
mod test_context;
pub mod timer;
mod timer_ext;

pub use context_namespaces::*;
pub use contextual::*;
pub use event::*;
pub use group::Group;
pub use multi::Multi;
pub(crate) use provides::ProvidedTraits;
pub use provides::Provides;
pub use routine::{
  RoutineBuilder, RoutineCompleted, RoutineCompletion, RoutineDefinition, RoutineStarted,
};
pub use service_context::*;
pub use state::{State, StateChanged};
pub use status_reporter::StatusReporter;
pub use system::*;
pub use system_container::*;
pub use system_context::*;
pub(crate) use system_group::*;
pub use system_handle::*;
#[cfg(any(test, feature = "testing"))]
pub use test_context::*;
pub use timer::*;
pub use timer_ext::*;

use std::any::TypeId;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::Debug;

use indexmap::IndexMap;

pub struct Systems {
  systems: IndexMap<u64, RefCell<SystemContainer>>,
  /// Each system's own type, and the traits it provides (see [`System::provides`]), to the latest system inserted
  /// with it
  type_to_id: HashMap<TypeId, u64>,
  id_to_types: HashMap<u64, Vec<TypeId>>,
  // Kept outside the cells, so a system can be named while it's borrowed (e.g. running a handler)
  id_to_name: HashMap<u64, &'static str>,
}

impl Systems {
  pub fn new() -> Self {
    Self {
      systems: IndexMap::new(),
      type_to_id: HashMap::new(),
      id_to_types: HashMap::new(),
      id_to_name: HashMap::new(),
    }
  }

  pub fn ids(&self) -> Vec<&u64> {
    self.systems.keys().collect()
  }

  pub fn names(&self) -> Vec<&'static str> {
    self
      .systems
      .keys()
      .filter_map(|id| self.name_of(id))
      .collect()
  }

  /// The system's name, readable even while the system is borrowed
  pub fn name_of(&self, system_id: &u64) -> Option<&'static str> {
    self.id_to_name.get(system_id).copied()
  }

  pub(crate) fn insert(&mut self, system: impl Into<SystemContainer>) {
    let system = system.into();
    let types: Vec<TypeId> = std::iter::once(system.type_id())
      .chain(system.provided_traits())
      .collect();
    for type_id in &types {
      self.type_to_id.insert(*type_id, system.id());
    }
    self.id_to_types.insert(system.id(), types);
    self.id_to_name.insert(system.id(), system.name());
    self.systems.insert(system.id(), RefCell::new(system));
  }

  /// Insert the system without entering its type in the type table, so lookups by type and routines don't find it
  pub(crate) fn insert_unlisted(&mut self, system: SystemContainer) {
    self.id_to_types.insert(system.id(), Vec::new());
    self.id_to_name.insert(system.id(), system.name());
    self.systems.insert(system.id(), RefCell::new(system));
  }

  pub(crate) fn remove(&mut self, system_id: u64) -> Option<RefCell<SystemContainer>> {
    let result = self.systems.shift_remove(&system_id);
    self.id_to_name.remove(&system_id);
    for type_id in self.id_to_types.remove(&system_id).unwrap_or_default() {
      // Only clear the mapping if it still points at this system, so removing an older system of the same type (or
      // providing the same trait) doesn't orphan a newer one.
      if self.type_to_id.get(&type_id) == Some(&system_id) {
        self.type_to_id.remove(&type_id);
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

  /// Borrow the system. `None` if there's no such system, or if it's already borrowed, which logs an error: the run
  /// loop borrows a system for as long as one of its handlers runs, so a system can't reach itself this way.
  pub fn get_by_id(&'_ self, system_id: &u64) -> Option<RefMut<'_, SystemContainer>> {
    self.borrow_by_id(system_id).unwrap_or_else(|busy| {
      log::error!(target: "frontbox::systems", "{busy}");
      None
    })
  }

  /// Borrow the system of type `T`. `None` if there isn't one, or if it's already borrowed, which logs an error (see
  /// [`get_by_id`](Self::get_by_id)).
  pub fn get_by_type<T: System + 'static>(&'_ self) -> Option<RefMut<'_, T>> {
    self.borrow_by_type::<T>().unwrap_or_else(|busy| {
      log::error!(target: "frontbox::systems", "{busy}");
      None
    })
  }

  /// Like [`get_by_id`](Self::get_by_id), but hands back the busy case for the caller to report
  pub(crate) fn borrow_by_id(
    &'_ self,
    system_id: &u64,
  ) -> Result<Option<RefMut<'_, SystemContainer>>, SystemBusy> {
    let Some(cell) = self.systems.get(system_id) else {
      return Ok(None);
    };
    cell.try_borrow_mut().map(Some).map_err(|_| SystemBusy {
      name: self.name_of(system_id).unwrap_or("<unnamed>"),
    })
  }

  /// Like [`get_by_type`](Self::get_by_type), but hands back the busy case for the caller to report
  pub(crate) fn borrow_by_type<T: System + 'static>(
    &'_ self,
  ) -> Result<Option<RefMut<'_, T>>, SystemBusy> {
    let Some(system_id) = self.type_to_id.get(&TypeId::of::<T>()) else {
      return Ok(None);
    };
    Ok(self.borrow_by_id(system_id)?.map(|system| {
      RefMut::map(system, |container| {
        container
          .downcast_mut::<T>()
          .expect("type_to_id mapping was incorrect")
      })
    }))
  }

  /// Like [`borrow_by_type`](Self::borrow_by_type), for the latest system providing the trait `I` (see
  /// [`System::provides`])
  pub(crate) fn borrow_by_trait<I: ?Sized + 'static>(
    &'_ self,
  ) -> Result<Option<RefMut<'_, I>>, SystemBusy> {
    let Some(system_id) = self.type_to_id.get(&TypeId::of::<I>()) else {
      return Ok(None);
    };
    // `I` might be a system's own type rather than a trait it provides, which this doesn't find
    Ok(
      self
        .borrow_by_id(system_id)?
        .and_then(|system| RefMut::filter_map(system, |container| container.cast_mut::<I>()).ok()),
    )
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
          .map(|(id, _)| (id, self.name_of(id).unwrap_or("<unnamed>")))
          .collect::<IndexMap<_, _>>(),
      )
      .finish()
  }
}

/// A system lookup found the system already borrowed: it's running a handler, or another lookup holds it
#[derive(Debug)]
pub(crate) struct SystemBusy {
  pub name: &'static str,
}

impl std::fmt::Display for SystemBusy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "System {} was looked up while already in use (it's running a handler, or something up the call chain holds \
       it). The lookup returns None.",
      self.name
    )
  }
}
