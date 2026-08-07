//! # Events
//! 
//! Frontbox systems receive events through the `on_event` handler.
//! 
//! ```rust
//! impl System for Example {
//!   fn on_event(&mut self, event: &dyn Signal, ctx: &Context) { }
//! }
//! ```
//! 
//! Events are typically handled by attempting a downcast into the expected type.
//! 
//! ```rust
//! impl System for Example {
//!   fn on_event(&mut self, event: &dyn Signal, ctx: &Context) {
//!     // detect if the event is of type `SwitchClosed`
//!     if let Some(e) = event.downcast_ref::<SwitchClosed>() {
//!       log::debug!("Switch {} was closed!", e.name);
//!     }
//! 
//!     // simple tests are also possible:
//!     let is_switch_closed = event.is::<SwitchClosed>();
//!   }
//! }
//! ```
//! 
//! Events are both something that the framework provides (e.g. switch open/closed) and something that can be defined by the end user. The only requirement is that values be thread safe (`Send + Sync`).
//! 
//! ```rust
//! // Events can simply be a body-less struct representing a typed thing
//! pub struct MyCustomThing;
//! 
//! // Events can also contain data
//! pub struct MyCustomThing2 {
//!   pub prop1: u8,
//!   pub prop2: String,
//! }
//! pub struct MyTupleLikeThing(i8, i8);
//! ```
//! 
//! #### Emitting Event
//! 
//! Events are broadcast to to all systems. While it's technically possible for every system to emit every event, in practice typically only a small handle of systems emit a particular event.
//! 
//! ```rust
//! ctx.emit(MyCustomThing2 { prop1: 4, prop2: "example".to_string() });
//! 
//! // ...
//! 
//! impl System for Example {
//!   fn on_event(&mut self, event: &dyn Signal, ctx: &Context) {
//!     if let Some(custom) = event.downcast_ref::<MyCustomThing2>() {
//!       log::debug!("Custom thing happened with {}, {}", custom.prop1, custom.prop2);
//!     }
//!   }
//! }
//! ```
//! 
//! ##### Event Layering
//! 
//! Sometimes systems receive lower level events (e.g. switch state changed) and process them into higher level events. These higher level events themselves get processed into game level events.
//! 
//! For example...
//! 
//! - The framework might emit a `SwitchClosed` event
//! - The `Trough` system interprets this and emits `TroughOccupancyChanged` and possibly `TroughFull`
//! - These trough level events are received by a game manager that may emit `PlayerTurnEnding`.

use std::any::Any;

/// An event is something which has happened in the world, such as a switch changing state, a game mode starting, a player scoring points, etc.
/// 
/// - Events names are written in the past tense
/// - Events can be any thread-safe value (blanket `impl` on `Any`)
/// 
/// # Examples
/// 
/// ```rust
/// // All of the following count as implementations of `Event`:
/// 
/// pub struct ExampleEvent1;
/// 
/// pub struct ExampleEvent2(pub id: u64);
/// 
/// pub struct ExampleEvent3 {
///   pub id: u64
/// }
/// ```
pub trait Event: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Send + Sync> Event for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

pub trait EventExt {
  /// Returns true if the event is of the given type `T`
  /// 
  /// ```rust
  /// # struct ExampleEvent1;
  /// # struct ExampleEvent2;
  /// 
  /// assert_true!(ExampleEvent1.is::<ExampleEvent1>());
  /// assert_false!(ExampleEvent1.is::<ExampleEvent2>());
  /// ```
  fn is<T: Any>(&self) -> bool;
  
  /// Returns a casted reference to `T`, if value is of type `T`. Otherwise returns `None`.
  /// 
  /// ```rust
  /// # struct ExampleEvent
  /// # let event: Any = ExampleEvent;
  /// if let Some(event) = event.downcast_ref::<ExampleEvent>() {
  ///   // do something with event
  /// }
  /// ```
  fn downcast_ref<T: Any>(&self) -> Option<&T>;
}

impl EventExt for dyn Event {
  fn is<T: Any>(&self) -> bool {
    self.as_any().is::<T>()
  }

  fn downcast_ref<T: Any>(&self) -> Option<&T> {
    self.as_any().downcast_ref::<T>()
  }
}

/// Event that happens when a new system is spawned. This is fired automatically by the framework.
#[derive(Debug, Clone, Copy)]
pub struct SystemSpawned {
  pub id: u64,
  pub parent_key: &'static str
}

impl SystemSpawned {
  pub fn new(id: u64, parent_key: &'static str) -> Self {
    Self { id, parent_key }
  }
}

/// Event that happens when an existing system is despawned. This is fired automatically by the framework.
#[derive(Debug, Clone, Copy)]
pub struct SystemDespawned {
  pub id: u64,
  pub parent_key: &'static str
}

impl SystemDespawned {
  pub fn new(id: u64, parent_key: &'static str) -> Self {
    Self { id, parent_key }
  }
}