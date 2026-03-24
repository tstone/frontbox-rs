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

  fn resolve(&mut self, name: &'static str, colors: Vec<(u64, Color)>) -> Color {
    if colors.len() == 0 {
      return Color::off();
    } else if colors.len() == 1 {
      return colors[0].1;
    } else {
      if let Some((last_system, elapsed)) = self.last_system.get_mut(name) {
        if *elapsed >= self.alternate_duration {
          // time to switch to the next system
          let current_index = colors
            .iter()
            // TODO: sort by system_id to be consistent
            .position(|(id, _)| id == last_system)
            .unwrap_or(0);
          let next_index = (current_index + 1) % colors.len();
          *last_system = colors[next_index].0;
          *elapsed = Duration::ZERO;
          return colors[next_index].1;
        } else {
          // keep showing the current system until it's time to switch
          return colors
            .iter()
            .find(|(id, _)| *id == *last_system)
            .map(|(_, color)| *color)
            .unwrap_or(Color::off());
        }
      } else {
        // no system has been shown yet, start with the first one
        self.last_system.insert(name, (colors[0].0, Duration::ZERO));
        return colors[0].1;
      }
    }
  }
}
