use std::time::Duration;

use tokio::sync::mpsc;

pub struct Watchdog {
  machine_to_watchdog_sender: mpsc::UnboundedSender<MachineToWatchdog>,
  watchdog_to_machine_receiver: mpsc::UnboundedReceiver<WatchdogToMachine>,
  enabled: bool,
}

impl Watchdog {
  pub fn new(tick_duration: Duration) -> Self {
    let (enablement_sender, mut enablement_receiver) = mpsc::unbounded_channel();
    let (watchdog_sender, watchdog_receiver) = mpsc::unbounded_channel();

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
          watchdog_sender.send(WatchdogToMachine::Tick).ok();
        }
      }
    });

    Self {
      machine_to_watchdog_sender: enablement_sender,
      watchdog_to_machine_receiver: watchdog_receiver,
      enabled: false,
    }
  }

  pub async fn read_tick(&mut self) -> Option<bool> {
    self
      .watchdog_to_machine_receiver
      .try_recv()
      .ok()
      .map(|_| true)
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

pub enum WatchdogToMachine {
  Tick,
}

pub enum MachineToWatchdog {
  Enable,
  Disable,
}
