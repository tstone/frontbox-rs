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