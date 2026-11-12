use std::time::Duration;

use crate::machine::driver_triggers::DriverTriggers;
use crate::machine::neuron::boot;
use crate::machine::serial_interface::{SerialError, SerialInterface};
use crate::machine::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

/// Handles serial port communication in an lightweight task
pub struct Neuron {
  io_port: SerialInterface,
  exp_port: SerialInterface,
  app_sender: mpsc::UnboundedSender<AppMessage>,
  machine_sender: mpsc::UnboundedSender<MachineMessage>,
  machine_receiver: mpsc::UnboundedReceiver<MachineMessage>,
  watchdog_interval: Duration,
  /// Failures in a row of the watchdog command being sent, to stop resending it after `WATCHDOG_ATTEMPTS`
  watchdog_failures: u32,
  driver_triggers: DriverTriggers,
}

impl Neuron {
  /// How many times in a row a watchdog command is sent before giving up until the next one
  const WATCHDOG_ATTEMPTS: u32 = 3;

  fn port_for(&mut self, port: MachinePort) -> &mut SerialInterface {
    match port {
      MachinePort::Io => &mut self.io_port,
      MachinePort::Exp => &mut self.exp_port,
    }
  }

  async fn process_messages(&mut self, msg: MachineMessage) {
    match msg {
      MachineMessage::WatchdogPing => {
        self
          .send_watchdog(
            WatchdogCommand::set(self.watchdog_interval),
            MachineMessage::WatchdogPing,
          )
          .await;
      }
      MachineMessage::WatchdogClear => {
        self
          .send_watchdog(WatchdogCommand::disable(), MachineMessage::WatchdogClear)
          .await;
      }
      MachineMessage::RefreshSwitchState => {
        self.refresh_switch_state().await;
      }
      MachineMessage::ConfigureDriver { driver_id, config } => {
        self.driver_triggers.cleared(driver_id);
        self
          .io_port
          .command(
            &ConfigureDriverCommand::new(driver_id, config),
            COMMAND_TIMEOUT,
          )
          .await
          .ok();
      }
      MachineMessage::UpdateDriverConfig { driver_id, config } => {
        self
          .io_port
          .command(
            &ConfigureDriverCommand::new(driver_id, config),
            COMMAND_TIMEOUT,
          )
          .await
          .ok();
        // DL disconnected the driver's switch, so put it back the way it was
        if let Some(trigger) = self.driver_triggers.restore(driver_id) {
          self.io_port.dispatch(&trigger).await;
        }
      }
      MachineMessage::TriggerDriver {
        driver_id,
        mode,
        switch,
      } => {
        self.driver_triggers.triggered(driver_id, mode, switch);
        self
          .io_port
          .dispatch(&TriggerDriverCommand::new(driver_id, mode, switch))
          .await;
      }
      MachineMessage::Dispatch { port, command } => {
        let port = self.port_for(port);
        port.dispatch(&*command).await;
      }
      MachineMessage::Command {
        port,
        command,
        timeout,
      } => {
        let port = self.port_for(port);
        port.command(&*command, timeout).await.ok();
      }
    }
  }

  fn handle_event(app_sender: &mpsc::UnboundedSender<AppMessage>, event: EventResponse) {
    match event {
      EventResponse::Switch { switch_id, state } => {
        app_sender
          .send(AppMessage::SwitchStateChange(switch_id, state))
          .ok();
      }
    }
  }

  fn flush_queued_events(&mut self) {
    for event in self.io_port.take_queued_events() {
      Self::handle_event(&self.app_sender, event);
    }
  }

  async fn refresh_switch_state(&mut self) {
    match self
      .io_port
      .query(&ReportSwitchesCommand, Duration::from_millis(125))
      .await
    {
      Ok(resp) => {
        if let SwitchReportResponse::SwitchReport { switches } = resp {
          // Make sure all switch events are queued with the app run loop before sending the switch state report
          self.flush_queued_events();
          self
            .app_sender
            .send(AppMessage::SyncSwitchStates(switches))
            .ok();
        }
      }
      _ => {}
    }
  }

  /// Send a watchdog command. If it fails or times out, `again` is queued to send it again behind whatever else is
  /// waiting, as the watchdog is critical to keeping the machine running, up to `WATCHDOG_ATTEMPTS` times in a row.
  async fn send_watchdog(&mut self, cmd: WatchdogCommand, again: MachineMessage) {
    match self.io_port.query(&cmd, Duration::from_millis(100)).await {
      // the port is gone, so there's nothing to send it on
      Err(SerialError::Lost) => {}
      Ok(WatchdogResponse::Failed) | Err(SerialError::Response(_)) => {
        self.watchdog_failures += 1;
        if self.watchdog_failures < Self::WATCHDOG_ATTEMPTS {
          let _ = self.machine_sender.send(again);
        } else {
          log::warn!(
            "🐶 The watchdog command failed {} times in a row, waiting for the next one",
            self.watchdog_failures
          );
          self.watchdog_failures = 0;
        }
      }
      Ok(_) => self.watchdog_failures = 0,
    }
  }

  /// Whether both ports are still connected. They're on the same USB connection, so losing either (e.g. because the
  /// cable was pulled) means the machine is gone.
  fn is_connected(&self) -> bool {
    self.io_port.is_connected() && self.exp_port.is_connected()
  }

  /// Read events and handle messages until a port is lost
  async fn run_connected(&mut self) {
    loop {
      tokio::select! {
        event = self.io_port.read_event() => match event {
          Ok(event) => Self::handle_event(&self.app_sender, event),
          Err(_) => return,
        },

        Some(msg) = self.machine_receiver.recv() => {
          self.process_messages(msg).await;
          self.flush_queued_events();
          if !self.is_connected() {
            return;
          }
        }
      }
    }
  }
}

impl Machine for Neuron {
  fn sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
    self.machine_sender.clone()
  }

  async fn on_pre_run(&mut self, state: &AppState) {
    boot::retry(async || boot::configure_drivers(&mut self.io_port, state).await)
      .await
      .unwrap_or_else(boot::boot_failed);
  }

  async fn run(&mut self) {
    self.run_connected().await;

    log::error!(
      "🔌 Lost the connection to the machine. Switches and drivers are unavailable until it restarts."
    );
    self.app_sender.send(AppMessage::MachineDisconnected).ok();

    // Nothing can reach the hardware any more. Keep draining what's sent so it doesn't pile up.
    while self.machine_receiver.recv().await.is_some() {}
  }
}

impl MachineBoot for Neuron {
  type Machine = Neuron;

  async fn boot(
    boot_config: BootConfig,
    app_sender: mpsc::UnboundedSender<AppMessage>,
  ) -> (Neuron, Hardware)
  where
    Self: Sized,
  {
    let (io_net_port_path, exp_port_path) = match boot_config.platform {
      Platform::Neuron {
        io_net_port_path,
        exp_port_path,
        ..
      } => (io_net_port_path, exp_port_path),
      _ => panic!(
        "Neuron machine being launched, but not configured to run as Neuron (should be impossible)!"
      ),
    };

    let app_config = AppConfig::from_boot_config(&boot_config);

    let mut io_port = SerialInterface::new(io_net_port_path)
      .await
      .expect("Failed to open IO NET port");
    log::info!("🥾 Opened IO NET port at {}", io_net_port_path);

    boot::retry_up_to(boot::MAINBOARD_HANDSHAKE_ATTEMPTS, async || {
      boot::handshake(&mut io_port).await
    })
    .await
    .unwrap_or_else(boot::boot_failed);

    // Verify user-configuration and load firmware/board versions
    let io_network = boot_config.io_network;
    let resolved_io_network =
      boot::retry(async || boot::resolve_io_network(&mut io_port, &io_network).await)
        .await
        .unwrap_or_else(boot::boot_failed);

    boot::verify_watchdog(&mut io_port).await;
    boot::configure_switches(&mut io_port, &io_network.switches).await;

    // Initialize switch context which Machine will use to maintain current state
    let initial_switch_state = boot::retry(async || boot::read_switch_states(&mut io_port).await)
      .await
      .unwrap_or_else(boot::boot_failed);
    let switch_lookup = SwitchLookup::new(io_network.switches, initial_switch_state);

    // open EXP port
    let mut exp_port = SerialInterface::new(exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", exp_port_path);

    let expansion_boards = Hardware::resolve_expansion_boards(&boot_config.exp_network.boards);
    boot::retry(async || boot::reset_expansion_boards(&mut exp_port, &expansion_boards).await)
      .await
      .unwrap_or_else(boot::boot_failed);
    boot::configure_led_ports(&mut exp_port, &expansion_boards).await;

    // Insert hardware definitions into store for systems to reference
    log::debug!("Initializing Store with hardware definitions");

    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();
    let neuron = Self {
      io_port,
      exp_port,
      app_sender,
      machine_sender,
      machine_receiver,
      watchdog_interval: app_config.watchdog_interval.unwrap(),
      watchdog_failures: 0,
      driver_triggers: DriverTriggers::default(),
    };

    let hardware = Hardware::new(
      switch_lookup,
      DriverLookup::new(io_network.drivers),
      LedLookup::new(&expansion_boards),
      resolved_io_network.boards,
      expansion_boards,
    );

    (neuron, hardware)
  }
}
