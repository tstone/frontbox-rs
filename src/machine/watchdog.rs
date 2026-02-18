use std::time::Duration;

use tokio::sync::mpsc;

use crate::machine::machine_command::MachineCommand;

pub struct Watchdog {
  machine_to_watchdog_sender: mpsc::UnboundedSender<MachineToWatchdog>,
  enabled: bool,
}

impl Watchdog {
  pub fn new(tick_duration: Duration, sender: mpsc::UnboundedSender<MachineCommand>) -> Self {
    let (enablement_sender, mut enablement_receiver) = mpsc::unbounded_channel();

    tokio::spawn(async move {
      let mut interval = tokio::time::interval(tick_duration);
      let mut enabled = false;

      loop {
        interval.tick().await;

        match enablement_receiver.try_recv() {
          Ok(MachineToWatchdog::Enable) => enabled = true,
          Ok(MachineToWatchdog::Disable) => enabled = false,
          _ => {}
        }

        if enabled {
          sender.send(MachineCommand::WatchdogTick).ok();
        }
      }
    });

    Self {
      machine_to_watchdog_sender: enablement_sender,
      enabled: false,
    }
  }

  pub fn enable(&mut self) {
    self
      .machine_to_watchdog_sender
      .send(MachineToWatchdog::Enable)
      .ok();
    self.enabled = true;
  }

  pub fn disable(&mut self) {
    self
      .machine_to_watchdog_sender
      .send(MachineToWatchdog::Disable)
      .ok();
    self.enabled = false;
  }
}

pub enum MachineToWatchdog {
  Enable,
  Disable,
}
