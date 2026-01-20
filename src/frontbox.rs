use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tokio::sync::mpsc;

use crate::mainboard_comms::{
  MainboardCommand, MainboardComms, MainboardConfig, MainboardIncoming,
};
use crate::prelude::*;

pub struct Frontbox {
  pub mainboard_config: MainboardConfig,
  pub io_network: IoNetwork,
}

impl Plugin for Frontbox {
  fn build(&self, app: &mut App) {
    let (command_tx, command_rx) = mpsc::channel::<MainboardCommand>(64);
    let (event_tx, event_rx) = mpsc::channel::<MainboardIncoming>(64);
    let mut mainboard = MainboardComms::new(self.mainboard_config.clone(), command_rx, event_tx);

    // TODO: separate boot from run

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

    // TODO: spawn entities for io network boards + child pins

    app
      .add_systems(Update, bridge_mainboard_events)
      .insert_resource(Mainboard::new(command_tx, event_rx));
  }
}

fn bridge_mainboard_events(mut mainboard: ResMut<Mainboard>, mut commands: Commands) {
  while let Some(event) = mainboard.receive() {
    commands.trigger(event);
  }
}
