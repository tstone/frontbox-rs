use std::any::Any;
use std::fmt::Debug;
use std::sync::atomic::AtomicU64;

use crate::prelude::*;

static LISTENER_ID: AtomicU64 = AtomicU64::new(0);

pub(crate) fn next_listener_id() -> u64 {
  LISTENER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub trait FrontboxEvent: Any + Debug + Send + Sync {
  fn as_any(&self) -> &dyn Any;
}

impl<T: Any + Debug + Send + Sync> FrontboxEvent for T {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

// --- Built-in events ---

/// Runs when a switch becomes closed (depressed)
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchClosed {
  pub switch: Switch,
}

impl SwitchClosed {
  pub fn new(switch: Switch) -> Box<SwitchClosed> {
    Box::new(Self { switch })
  }
}

/// Runs when a switch becomes open (released)
#[derive(Debug)]
#[allow(unused)]
pub struct SwitchOpened {
  pub switch: Switch,
}

impl SwitchOpened {
  pub fn new(switch: Switch) -> Box<SwitchOpened> {
    Box::new(Self { switch })
  }
}

/// Runs when a timer finishes
#[derive(Debug)]
#[allow(unused)]
pub struct TimerComplete {
  pub name: &'static str,
}

impl TimerComplete {
  pub fn new(name: &'static str) -> Box<TimerComplete> {
    Box::new(Self { name })
  }
}

/// Runs when a game starts
#[derive(Debug)]
pub struct GameStarted;

impl GameStarted {
  pub fn new() -> Box<GameStarted> {
    Box::new(Self)
  }
}

/// Runs when a game ends
#[derive(Debug)]
pub struct GameEnded;

impl GameEnded {
  pub fn new() -> Box<GameEnded> {
    Box::new(Self)
  }
}

/// Runs when a player is added
#[derive(Debug)]
#[allow(unused)]
pub struct PlayerAdded {
  pub player_count: u8,
}

impl PlayerAdded {
  pub fn new(player_count: u8) -> Box<PlayerAdded> {
    Box::new(Self { player_count })
  }
}
