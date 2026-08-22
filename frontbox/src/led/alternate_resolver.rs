use crate::prelude::*;
use std::collections::HashMap;

pub struct AlternateResolver {
  states: HashMap<LedAddress, AlternatingEntry>,
  // how much time must pass before we switch to the next system in a conflict
  alternate_duration: Duration,
}

impl AlternateResolver {
  pub fn new() -> Self {
    Self {
      states: HashMap::new(),
      alternate_duration: Duration::from_millis(215), // TODO: make this configurable
    }
  }

  pub fn reset(&mut self) {
    self.states.clear();
  }

  pub fn accumulate(&mut self, delta: Duration) {
    for (_, entry) in self.states.iter_mut() {
      entry.acc_duration += delta;
    }
  }

  pub fn resolve(&mut self, led: LedAddress, colors: Vec<Rgba<u8>>) -> Rgba<u8> {
    // check first if we have an existing system for this led, and if it's still valid
    if let Some(entry) = self.states.get_mut(&led) {
      if entry.colors == colors {
        if entry.acc_duration >= self.alternate_duration {
          entry.acc_duration = Duration::ZERO;
          entry.idx += 1;
          if entry.idx == entry.colors.len() {
            entry.idx = 0;
          }
        }
        return entry.colors[entry.idx];
      } else {
        // different colors, reset the duration and move to the next color
        entry.acc_duration = Duration::ZERO;
        entry.idx = 0;
        entry.colors = colors;
        return entry.colors[entry.idx];
      }
    } else {
      log::trace!("Creating new alternating state");
      // no existing record of colors, create a new one
      self.states.insert(
        led.clone(),
        AlternatingEntry {
          colors: colors.clone(),
          acc_duration: Duration::from_millis(0),
          idx: 0,
        },
      );
      return colors[0];
    }
  }
}

struct AlternatingEntry {
  colors: Vec<Rgba<u8>>,
  acc_duration: Duration,
  idx: usize,
}
