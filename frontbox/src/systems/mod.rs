//! # Systems
//! 
//! <div class="warning">Stability Level: High</div>
//! 
//! The heart of Frontbox is a `System`. Almost everything is a System: game modes, credit modes, sound mixer, even the display. Systems interact with the world through events. Systems are just Rust structs, which can manage their own state and be extended with private functions. They have a handful of callback type methods, including general lifecycle `on_startup` and `on_shutdown` handlers.
//! 
//! ```rust
//! struct ExampleSystem {
//!   private_data: u64,
//! }
//! 
//! impl System for ExampleSystem {
//!   fn on_startup(&mut self, ctx: &Context) {
//!     // <do cool stuff here>
//!   }
//! }
//! ```
//! 
//! ## What Systems Do
//! 
//! - [Emit events](crate::systems::event)
//! - [Interrupt events](crate::systems::event_interrupts)
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
//! - `on_startup`
//! - `on_deactivate`
//! - `on_reactivate`
//! - `on_shutdown`
//! 
//! #### Startup
//! 
//! Systems can be given on startup, and will be started automatically, or dynamically spawned at runtime. Likewise, running systems can be despawned or replaced.
//! 
//! ```rust
//! // Start a new system
//! ctx.spawn_system(ExampleSystem::new());
//! 
//! // Stop the current system and immediately spawn a replacement
//! ctx.replace_self(ExampleSystem::new());
//! 
//! // Just stop the current system
//! ctx.despawn_self();
//! ```
//! 
//! ### Active
//! 
//! By default, all systems spawned are active. Systems can be despawned, which removes them entirely, but sometimes it's necessary to keep a system around, having it automatically become active in certain situations. Frontbox supports this feature by way of the `is_active() -> bool` handler.
//! 
//! If `is_active` returns `false`, the framework will by skip all other handlers (the ones starting with `on_*`). Within `is_active`, only read access to `self` and `Context` is provided.
//! 
//! ```rust
//! // Example system is only active during a game
//! impl System for ExampleSystem {
//!   fn is_active(&self, ctx: &Context) -> bool {
//!     ctx.is_game_started()
//!   }
//! }
//! ```
//! 
//! Systems active state can also be controled by a [parent group](mod@crate::systems::system_group).
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
//! 1. `get::<S>` - Returns a mutable reference to `Option<S>`
//! 2. `expect::<S>` - Returns mutable reference to `S`, also panics if it does not exist
//! 
//! ```rust
//! // safe but verbose
//! if let Some(trough) = ctx.systems.get::<TroughSystem() {
//!   trough.eject()
//! }
//! 
//! // unsafe when you absolutely know it is present 
//! ctx.systems.expect::<TroughSystem>().eject();
//! ```

mod context;
mod context_base;
mod contextual;
pub mod cue;
mod cue_timeline_builder;
mod system;
mod system_container;
pub mod system_group;
mod system_handle;
mod systems_context;
pub mod event;
pub mod event_interrupts;

pub use context::*;
pub use context_base::*;
pub use contextual::*;
pub use cue::*;
pub use cue_timeline_builder::*;
pub use system::*;
pub use system_container::*;
pub use system_group::*;
pub use system_handle::*;
pub use systems_context::*;
pub use event::*;
pub use event_interrupts::InterruptResult;

use std::any::TypeId;
use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::fmt::Debug;

pub struct Systems {
  systems: HashMap<u64, RefCell<SystemContainer>>,
  type_to_id: HashMap<TypeId, u64>,
  id_to_type: HashMap<u64, TypeId>,
}

impl Systems {
  pub fn new() -> Self {
    Self {
      systems: HashMap::new(),
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
    let result = self.systems.remove(&system_id);
    if let Some(system_type) = self.id_to_type.remove(&system_id) {
      self.type_to_id.remove(&system_type);
    }
    result
  }

  pub fn get_id<T: System + 'static>(&self) -> Option<u64> {
    let type_id = TypeId::of::<T>();
    self.type_to_id.get(&type_id).copied()
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

  pub(crate) fn values(&self) -> impl Iterator<Item = &RefCell<SystemContainer>> {
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
          .collect::<HashMap<_, _>>(),
      )
      .finish()
  }
}
