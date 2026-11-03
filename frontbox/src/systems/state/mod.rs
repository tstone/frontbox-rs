//! # States
//!
//! A state is a small value, usually an enum, that systems branch on: which part of a turn it is, which mode is
//! running. The framework holds it, so reading it doesn't mean looking up the system that sets it, and the framework
//! broadcasts [`StateChanged<S>`] whenever it changes.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! #[derive(State, Serialize, Default, Clone, PartialEq)]
//! enum Multiball {
//!   #[default]
//!   Unlit,
//!   Lit,
//!   Running,
//! }
//! ```
//!
//! A state always has a value: until it's first set, it reads as its `Default`.
//!
//! ## Setting
//!
//! A state belongs to the first system to set it, in that system's group. Other systems can read it, but only its
//! owner can set it; anyone else trying is ignored, with a warning. When the owner is despawned, the state goes back
//! to its default, and the next system to set it owns it.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(State, Serialize, Default, Clone, PartialEq)]
//! # enum Multiball { #[default] Unlit, Lit, Running }
//! # fn example(ctx: &SystemContext) {
//! ctx.set_state(Multiball::Running);
//! # }
//! ```
//!
//! Like emitting an event, setting a state takes effect in queue order: events emitted before it are handled while the
//! old value still holds, and the system setting it reads the old value until then. Once it's set, [`StateChanged<S>`]
//! is broadcast, unless the value didn't change.
//!
//! ## Reading
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(State, Serialize, Default, Clone, PartialEq)]
//! # enum Multiball { #[default] Unlit, Lit, Running }
//! # fn example(ctx: &SystemContext) {
//! let multiball = ctx.state::<Multiball>();
//! let running = ctx.in_state(Multiball::Running);
//! # }
//! ```
//!
//! ## Groups
//!
//! Each group has its own states. A system reads the value set in its own group, then the root's, the same nearness
//! as looking up a system. Since every player has their own group, a state set by a system in a player's group is that
//! player's: it's still there when their next turn comes around, and goes when the group is despawned. A root system
//! can't see a state set in a group.

mod states;
mod stored_state;
#[cfg(test)]
mod tests;

pub(crate) use states::*;
pub(crate) use stored_state::*;

use std::any::{Any, type_name};

use crate::prelude::*;

/// A value the framework holds for systems to read, and broadcasts a [`StateChanged<S>`] for when it changes. See
/// [states](crate::systems::state). Derivable.
pub trait State: Any + Send + Sync + Clone + PartialEq + Default + serde::Serialize {}

/// Broadcast when a state is set to a new value, or goes back to its default because its owner was despawned
///
/// ```rust
/// # use frontbox::prelude::*;
/// # #[derive(State, Serialize, Default, Clone, PartialEq)]
/// # enum Multiball { #[default] Unlit, Lit, Running }
/// # struct Display;
/// # impl Display { fn show_lock_lit(&mut self, ctx: &SystemContext) {} }
/// impl System for Display {
///   fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
///     if let Some(changed) = event.downcast_ref::<StateChanged<Multiball>>() {
///       if changed.to == Multiball::Lit {
///         self.show_lock_lit(ctx);
///       }
///     }
///   }
/// }
/// ```
pub struct StateChanged<S: State> {
  pub from: S,
  pub to: S,
}

// by hand: the derive doesn't handle generics
impl<S: State> Event for StateChanged<S> {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn to_json(&self) -> Option<serde_json::Value> {
    Some(serde_json::json!({
      "name": type_name::<S>(),
      "from": serde_json::to_value(&self.from).ok()?,
      "to": serde_json::to_value(&self.to).ok()?,
    }))
  }
}
