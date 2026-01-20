use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tokio::sync::mpsc;

use crate::protocol::configure_hardware::SwitchReporting;
use crate::{FastPlatform, MainboardCommand, MainboardComms, MainboardConfig};

#[derive(Resource)]
pub struct Mainboard {
  pub command_tx: mpsc::Sender<MainboardCommand>,
}

impl Mainboard {
  pub fn enable_watchdog(&self) {
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(true));
  }

  pub fn disable_watchdog(&self) {
    let _ = self.command_tx.try_send(MainboardCommand::Watchdog(false));
  }
}

pub struct MainboardPlugin {
  pub io_net_port_path: &'static str,
  pub exp_port_path: &'static str,
  pub platform: FastPlatform,
  pub switch_reporting: Option<SwitchReporting>,
}

impl Plugin for MainboardPlugin {
  fn build(&self, app: &mut App) {
    let (tx, rx) = mpsc::channel::<MainboardCommand>(64);
    let mut mainboard = MainboardComms::new(
      MainboardConfig {
        io_net_port_path: self.io_net_port_path,
        exp_port_path: self.exp_port_path,
        platform: self.platform.clone(),
        switch_reporting: self.switch_reporting.clone(),
      },
      rx,
    );

    // start serial communication in separate thread
    std::thread::spawn(move || {
      let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

      runtime.block_on(async move {
        mainboard.run().await;
      });
    });

    app.insert_resource(Mainboard { command_tx: tx });
  }
}
