pub struct ExampleMode {
  hits: u8,
}

impl Mode for ExampleMode {
  fn on_switch(&mut self, switch: &Switch, game: &Game<CustomState>) {
    if switch.name == Switches::EXAMPLE_TARGET {
      self.hits += 1;
      game.player.add_score(500);
    }
  }

  fn get_state(&self, game: &Game<CustomState>) -> ModeState {
    if self.hits >= 5 {
      ModeState::Complete
    } else {
      ModeState::Active
    }
  }
}

pub enum LaneQualification {
  Left,
  Right,
  Always,
}

pub struct JetsQualificationMode {
  lane_qualification: LaneQualification,
  left_qualified: bool,
  right_qualified: bool,
  left_hits: u8,
  right_hits: u8,
}

impl Mode for JetsQualificationMode {
  fn on_switch(&mut self, switch: &Switch, player: &mut Player, machine: &Machine) -> ModeState {
    if switch.name == Switches::PASS_LANE && self.lane_qualification == LaneQualification::Left {
      self.left_qualified = true;
      self.start_timer("qualification", Duration::from_secs(30));
    } else if switch.name == Switches::PASS_LANE
      && self.lane_qualification == LaneQualification::Right
    {
      self.right_qualified = true;
      self.start_timer("qualification", Duration::from_secs(30));
    } else if self.left_qualified && switch.name == Switches::LEFT_INLANE {
      player.add_score(1000);
      self.left_hits += 1;

      if self.lane_qualification != LaneQualification::Always {
        self.left_qualified = false;
      }
    } else if self.right_qualified && switch.name == Switches::RIGHT_INLANE {
      player.add_score(1000);
      self.right_hits += 1;

      if self.lane_qualification != LaneQualification::Always {
        self.right_qualified = false;
      }
    }

    if self.left_hits >= 3 && self.right_hits >= 3 {
      ModeState::Complete
    } else {
      ModeState::Active
    }
  }
}

impl ModeTimer for JetsQualificationMode {
  fn on_timer_complete(&mut self, timer: &'static str) {
    match timer {
      "qualification" => {
        self.left_qualified = false;
        self.right_qualified = false;
      }
      _ => {}
    }
  }
}

impl ModeInit for JetsQualificationMode {
  fn on_init(&mut self, game: Game<Godzilla>) {
    if game.state.jet_completions > 0 {
      self.left_qualified = false;
      self.right_qualified = false;
      self.lane_qualification = LaneQualification::Left;
    } else {
      self.left_qualified = true;
      self.right_qualified = true;
      self.lane_qualification = LaneQualification::Always;
    }
  }
}

impl ModeComplete for JetsQualificationMode {
  fn on_complete(&self, player: &mut Player, machine: &Machine) {
    player.add_score(100000);
  }
}

impl Indicators for JetsQualificationMode {
  fn active_leds(&self, player: &Player, machine: &Machine) -> Vec<LedConfiguration> {
    let mut configs = Vec::new();

    if self.left_qualified && self.left_hits < 3 {
      configs.push(LedConfiguration::Solid {
        led: LEDS::LEFT_INLANE,
        color: Color::RED,
      });
    }

    if self.right_qualified && self.right_hits < 3 {
      configs.push(LedConfiguration::Solid {
        led: LEDS::RIGHT_INLANE,
        color: Color::RED,
      });
    }

    if (!self.left_qualified && self.lane_qualification == LaneQualification::Left)
      || (!self.right_qualified && self.lane_qualification == LaneQualification::Right)
    {
      configs.push(LedConfiguration::Solid {
        led: LEDS::PASS_LANE,
        color: Color::RED,
      });
    }

    configs
  }
}

pub enum ModeState {
  Active,
  ActiveExclusive,
  Complete,
}
