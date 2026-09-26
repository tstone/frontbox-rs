use std::time::Duration;

use crate::machine::serial_interface::*;
use crate::prelude::app_message::AppMessage;
use crate::prelude::neuron::boot;
use crate::prelude::*;
use fast_protocol::*;
use tokio::sync::mpsc;

/// Handles serial port communication in an lightweight task
pub(crate) struct Neuron {
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
        self.send_watchdog_ping().await;
      }
      MachineMessage::Dispatch { port, command } => {
        let port = self.port_for(port);
        port.dispatch(&*command).await;
      }
      MachineMessage::Request {
        port,
        command,
        timeout,
      } => {
        let port = self.port_for(port);
        port.request_any(&*command, timeout).await.ok();
      }
    }
  }

  pub fn handle_switch_event(&mut self, switch_id: usize, state: SwitchState) {
    self
      .app_sender
      .send(AppMessage::SwitchStateChange(switch_id, state))
      .ok();

    // let switch = self.switches.by_id(&switch_id).cloned();

    // if let Some(switch) = switch {
    //   // App needs to update switch state in the store before sending out the event

    //   if matches!(state, SwitchState::Closed) {
    //     log::debug!(target: "frontbox::switches", "🎚️  Switch {} closed", switch.name);
    //     let event = SwitchClosed::new(switch);
    //     self
    //       .app_sender
    //       .send(AppMessage::EmitEvent(EventBox::new(event)))
    //       .ok();
    //   } else {
    //     log::debug!(target: "frontbox::switches", "🎚️  Switch {} opened", switch.name);
    //     let event = SwitchOpened::new(switch);
    //     self
    //       .app_sender
    //       .send(AppMessage::EmitEvent(EventBox::new(event)))
    //       .ok();
    //   }
    // } else {
    //   // TODO: Move this handling into the app run loop
    //   // Report as native board/switch id since this is the easiest way to figure out current switch wiring
    //   match self.get_native_switch_id(switch_id) {
    //     Some((board_id, local_id)) => {
    //       log::warn!(
    //         target: "frontbox::switches",
    //         "Received event for unknown switch -- board: {}, id: {} -- {:?}",
    //         board_id,
    //         local_id,
    //         state
    //       );
    //       return;
    //     }
    //     None => {
    //       log::warn!(
    //         target: "frontbox::switches",
    //         "Received event for unknown switch on unknown board {} -- {:?}",
    //         switch_id,
    //         state
    //       );
    //     }
    //   }
    //   return;
    // }
  }

  pub async fn send_watchdog_ping(&mut self) {
    match self
      .io_port
      .request(
        &WatchdogCommand::set(self.watchdog_interval),
        Duration::from_millis(200),
      )
      .await
    {
      // try again
      Ok(WatchdogResponse::Failed) => {
        let _ = self.machine_sender.send(MachineMessage::WatchdogPing);
      }
      _ => {}
    }
  }

  //   fn get_native_switch_id(&self, switch_id: usize) -> Option<(usize, usize)> {
  //     let mut offset: usize = 0;
  //     for (index, board) in self.io_network.iter().enumerate() {
  //       if switch_id < (board.switch_count as usize) + offset {
  //         let native_switch_id = switch_id - offset;
  //         return Some((index, native_switch_id));
  //       }
  //       offset += board.switch_count as usize;
  //     }
  //     None
  //   }
}

impl Machine for Neuron {
  fn sender(&self) -> mpsc::UnboundedSender<MachineMessage> {
    self.machine_sender.clone()
  }

  async fn run(&mut self) {
    loop {
      tokio::select! {
        Some(event) = self.io_port.read_event() => {
          match event {
            EventResponse::Switch { switch_id, state } => {
              self.handle_switch_event(switch_id, state);
            }
          }
        }

        Some(msg) = self.machine_receiver.recv() => {
          self.process_messages(msg).await;
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
    let app_config = AppConfig::from_boot_config(&boot_config);

    let mut io_port = SerialInterface::new(boot_config.io_net_port_path)
      .await
      .expect("Failed to open IO NET port");
    log::info!("🥾 Opened IO NET port at {}", boot_config.io_net_port_path);

    boot::run(&mut io_port).await;

    // Verify user-configuration and load firmware/board versions
    let io_network = boot_config.io_network;
    let resolved_io_network = Hardware::resolve_io_network(&mut io_port, &io_network).await;

    boot::verify_watchdog(&mut io_port).await;
    boot::configure_switches(&mut io_port, &io_network.switches).await;

    // Initialize switch context which Machine will use to maintain current state
    let initial_switch_state = Hardware::get_initial_switch_states(&mut io_port).await;
    let switch_lookup = SwitchLookup::new(io_network.switches, initial_switch_state);

    // open EXP port
    let mut exp_port = SerialInterface::new(boot_config.exp_port_path)
      .await
      .expect("Failed to open EXP port");
    log::info!("🥾 Opened EXP port at {}", boot_config.exp_port_path);

    let expansion_boards = Hardware::resolve_expansion_boards(&boot_config.exp_network.boards);
    Hardware::reset_expansion_boards(&mut exp_port, &expansion_boards).await;
    Hardware::configure_led_ports(&mut exp_port, &expansion_boards).await;

    // Insert hardware definitions into store for systems to reference
    log::debug!("Initializing Store with hardware definitions");

    let (machine_sender, machine_receiver) = mpsc::unbounded_channel::<MachineMessage>();
    let neuron = Self {
      io_port,
      exp_port,
      app_sender,
      machine_sender,
      machine_receiver,
      watchdog_interval: app_config.watchdog_interval + Duration::from_millis(250), // add some buffer to account for latency in sending
    };

    let hardware = Hardware::new(
      switch_lookup,
      DriverLookup::new(io_network.drivers),
      LedLookup::new(&expansion_boards),
      resolved_io_network.boards,
      expansion_boards,
    );

    // TODO: what if I didn't do this? should this really be done on start-up?
    // Hardware::configure_drivers(&mut io_port, &boot_snapshot).await;

    (neuron, hardware)
  }
}
