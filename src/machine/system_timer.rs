use std::time::Duration;

use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;

pub struct SystemTimer {
  target: Duration,
  accumulated: Duration,
  mode: TimerMode,
}

impl SystemTimer {
  pub fn new(target: Duration, mode: TimerMode) -> Self {
    Self {
      target,
      mode,
      accumulated: Duration::from_secs(0),
    }
  }

  pub fn tick(&mut self, delta: &Duration) -> bool {
    self.accumulated += *delta;
    if self.accumulated >= self.target {
      self.accumulated = self.accumulated - self.target;
      true
    } else {
      false
    }
  }

  pub fn mode(&self) -> &TimerMode {
    &self.mode
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerMode {
  OneShot,
  Repeating,
}

pub fn run_system_timers(tick: Duration, sender: mpsc::UnboundedSender<MachineCommand>) {
  let mut timer_interval = tokio::time::interval(tick);

  tokio::spawn(async move {
    loop {
      timer_interval.tick().await;
      sender.send(MachineCommand::SystemTick).ok();
    }
  });
}
