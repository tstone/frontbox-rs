use crate::prelude::*;

pub enum SystemMessage {
  SpawnSystem(Box<dyn System>),
  ReplaceSystem(u64, Box<dyn System>),
  DespawnSystem(u64),
  ClearTimer(u64, &'static str),
  SetTimer(u64, &'static str, Duration, TimerMode),
}
