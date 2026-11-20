use frontbox_core::prelude::*;

/// Accepts 3 letter initials from the player. Nothing shows them yet: see the initials entry item in `TODO.md`.
pub struct InitialsEntrySystem {
  title: String,
  name: String,
  decrement_switches: SwitchQ,
  increment_switches: SwitchQ,
  accept_switches: SwitchQ,
}

impl InitialsEntrySystem {
  /// **title** - The title to display above the initials entry field (e.g. "Player 1")
  pub fn new(
    title: impl Into<String>,
    decrement_switches: SwitchQ,
    increment_switches: SwitchQ,
    accept_switches: SwitchQ,
  ) -> Self {
    Self {
      title: title.into(),
      name: "A".to_string(),
      decrement_switches,
      increment_switches,
      accept_switches,
    }
  }

  fn decrement(&mut self) {
    let mut last_char = self.name.pop().unwrap_or(' ') as u8;
    if last_char == 32 {
      last_char = 95; // wrap around to '_'
    } else if last_char == 91 {
      last_char = 57; // skip to '9'
    } else if last_char == 48 {
      last_char = 90; // skip to 'Z'
    } else if last_char == 65 {
      last_char = 32; // skip to ' '
    } else {
      last_char -= 1;
    }
    self.name.push(last_char as char);
  }

  fn increment(&mut self) {
    let mut last_char = self.name.pop().unwrap_or(' ') as u8;
    if last_char == 32 {
      last_char = 65; // skip to 'A'
    } else if last_char == 90 {
      last_char = 48; // skip to '0'
    } else if last_char == 57 {
      last_char = 91; // skip to '['
    } else if last_char == 95 {
      last_char = 32; // skip to ' '
    } else {
      last_char += 1;
    }
    self.name.push(last_char as char);
  }

  /// Accept the letter being chosen, and move on to the next or finish with the third
  fn accept(&mut self, ctx: &SystemContext) {
    if self.name.len() < 3 {
      self.name.push('A');
      return;
    }

    log::info!(target: "frontbox::display", "Initials entered for {}: {}", self.title, self.name);
    ctx.emit(InitialsEntered {
      initials: self.name.clone(),
    });
    ctx.systems.despawn_self();
  }
}

impl System for InitialsEntrySystem {
  fn description() -> Option<&'static str> {
    Some("Lets a player enter their 3 initials, such as for a high score")
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if self.decrement_switches.matches(&event.switch) {
        self.decrement();
      } else if self.increment_switches.matches(&event.switch) {
        self.increment();
      } else if self.accept_switches.matches(&event.switch) {
        self.accept(ctx);
      }
    }
  }
}

#[derive(serde::Serialize, Event)]
pub struct InitialsEntered {
  pub initials: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn increment_a() {
    let mut system = InitialsEntrySystem::new(
      "Test".to_string(),
      SwitchQ::name("left_flipper"),
      SwitchQ::name("right_flipper"),
      SwitchQ::name("action"),
    );

    assert_eq!(system.name, "A");
    system.increment();
    assert_eq!(system.name, "B");
    system.increment();
    assert_eq!(system.name, "C");
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    assert_eq!(system.name, "G");
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment(); // L
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    assert_eq!(system.name, "P");
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    assert_eq!(system.name, "Y");
    system.increment();
    assert_eq!(system.name, "Z");
    system.increment();
    assert_eq!(system.name, "0");
    system.increment();
    assert_eq!(system.name, "1");
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    system.increment();
    assert_eq!(system.name, "9");
    system.increment();
    assert_eq!(system.name, "[");
    system.increment();
    assert_eq!(system.name, "\\");
    system.increment();
    assert_eq!(system.name, "]");
    system.increment();
    assert_eq!(system.name, "^");
    system.increment();
    assert_eq!(system.name, "_");
    system.increment();
    assert_eq!(system.name, " ");
    system.increment();
    assert_eq!(system.name, "A");
  }

  #[test]
  fn decrement_a() {
    let mut system = InitialsEntrySystem::new(
      "Test".to_string(),
      SwitchQ::name("left_flipper"),
      SwitchQ::name("right_flipper"),
      SwitchQ::name("action"),
    );

    assert_eq!(system.name, "A");
    system.decrement();
    assert_eq!(system.name, " ");
    system.decrement();
    assert_eq!(system.name, "_");
    system.decrement();
    assert_eq!(system.name, "^");
    system.decrement();
    assert_eq!(system.name, "]");
    system.decrement();
    assert_eq!(system.name, "\\");
    system.decrement();
    assert_eq!(system.name, "[");
    system.decrement();
    assert_eq!(system.name, "9");
    system.decrement();
    assert_eq!(system.name, "8");
    system.decrement();
    system.decrement();
    system.decrement();
    system.decrement();
    system.decrement();
    system.decrement();
    system.decrement();
    assert_eq!(system.name, "1");
    system.decrement();
    assert_eq!(system.name, "0");
    system.decrement();
    assert_eq!(system.name, "Z");
  }
}
