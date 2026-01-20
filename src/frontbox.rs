use bevy_app::prelude::*;
use tokio::sync::mpsc;

use crate::mainboard_comms::{MainboardCommand, MainboardComms, MainboardConfig};
use crate::prelude::*;

pub struct Frontbox {
  pub mainboard_config: MainboardConfig,
}

impl Plugin for Frontbox {
  fn build(&self, app: &mut App) {
    let (tx, rx) = mpsc::channel::<MainboardCommand>(64);
    let mut mainboard = MainboardComms::new(self.mainboard_config.clone(), rx);

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

    app.insert_resource(Mainboard {
      command_tx: tx,
      watchdog_enabled: false,
    });
  }
}
