use crate::prelude::*;
use std::collections::HashMap;

use crate::prelude::LedResolver;

pub struct AlternateResolver {
  last_system: HashMap<&'static str, (u64, Duration)>,
  // how much time must pass before we switch to the next system in a conflict
  alternate_duration: Duration,
}

impl AlternateResolver {
  pub fn new() -> Self {
    Self {
      last_system: HashMap::new(),
      alternate_duration: Duration::from_millis(225), // TODO: make this configurable
    }
  }
}

impl LedResolver for AlternateResolver {
  fn reset(&mut self) {
    self.last_system.clear();
  }

  fn tick(&mut self, delta: Duration) {
    for (_, (_, elapsed)) in self.last_system.iter_mut() {
      *elapsed += delta;
    }
  }

  fn resolve(&mut self, name: &'static str, states: Vec<(u64, LedState)>) -> LedState {
    if states.len() == 0 {
      return LedState::Off;
    } else if states.len() == 1 {
      return states[0].1.clone();
    } else {
      if let Some((last_system, elapsed)) = self.last_system.get_mut(name) {
        if *elapsed >= self.alternate_duration {
          // time to switch to the next system
          let current_index = states
            .iter()
            // TODO: sort by system_id to be consistent
            .position(|(id, _)| id == last_system)
            .unwrap_or(0);
          let next_index = (current_index + 1) % states.len();
          *last_system = states[next_index].0;
          *elapsed = Duration::ZERO;
          return states[next_index].1.clone();
        } else {
          // keep showing the current system until it's time to switch
          return states
            .iter()
            .find(|(id, _)| *id == *last_system)
            .map(|(_, state)| state.clone())
            .unwrap_or(LedState::Off);
        }
      } else {
        // no system has been shown yet, start with the first one
        self.last_system.insert(name, (states[0].0, Duration::ZERO));
        return states[0].1.clone();
      }
    }
  }
}
