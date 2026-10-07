use std::time::Duration;

use crate::machine::neuron::boot;
use crate::machine::serial_interface::SerialInterface;
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
}

impl Neuron {
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
          .send_watchdog(WatchdogCommand::set(self.watchdog_interval))
          .await;
      }
      MachineMessage::WatchdogClear => {
        self.send_watchdog(WatchdogCommand::disable()).await;
      }
      MachineMessage::RefreshSwitchState => {
        self.refresh_switch_state().await;
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
      .query(&ReportSwitchesCommand, Duration::from_secs(1))
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

  async fn send_watchdog(&mut self, cmd: WatchdogCommand) {
    match self.io_port.query(&cmd, Duration::from_millis(200)).await {
      // try again
      Ok(WatchdogResponse::Failed) => {
        let _ = self.machine_sender.send(MachineMessage::WatchdogPing);
      }
      _ => {}
    }
  }
}

impl Machine for Neuron {
  fn sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
    self.machine_sender.clone()
  }

  async fn on_pre_run(&mut self, snapshot: &BootSnapshot) {
    boot::configure_drivers(&mut self.io_port, &snapshot).await;
  }

  async fn run(&mut self) {
    loop {
      tokio::select! {
        Some(event) = self.io_port.read_event() => {
          Self::handle_event(&self.app_sender, event);
        }

        Some(msg) = self.machine_receiver.recv() => {
          self.process_messages(msg).await;
          self.flush_queued_events();
        }
      }
    }
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

    boot::handshake(&mut io_port).await;

    // Verify user-configuration and load firmware/board versions
    let io_network = boot_config.io_network;
    let resolved_io_network = boot::resolve_io_network(&mut io_port, &io_network).await;

    boot::verify_watchdog(&mut io_port).await;
    boot::configure_switches(&mut io_port, &io_network.switches).await;

    // Initialize switch context which Machine will use to maintain current state
    let initial_switch_state = boot::get_initial_switch_states(&mut io_port).await;
    let switch_lookup = SwitchLookup::new(io_network.switches, initial_switch_state);

    // open EXP port
    let mut exp_port = SerialInterface::new(exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", exp_port_path);

    let expansion_boards = Hardware::resolve_expansion_boards(&boot_config.exp_network.boards);
    boot::reset_expansion_boards(&mut exp_port, &expansion_boards).await;
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
      watchdog_interval: app_config.watchdog_interval.unwrap() + Duration::from_millis(250), // add some buffer to account for latency in sending
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
