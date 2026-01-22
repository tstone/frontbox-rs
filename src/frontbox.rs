use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tokio::sync::{broadcast, mpsc};

use crate::mainboard_io::{MainboardCommand, MainboardIncoming};
use crate::prelude::*;
use crate::protocol::FastResponse;

pub struct Frontbox {
  pub mainboard: Mainboard,
  pub io_network: IoNetwork,
}

impl Plugin for Frontbox {
  fn build(&self, app: &mut App) {
    let link = MainboardLink {
      command_tx: self.mainboard.tx(),
      event_rx: self.mainboard.subscribe(),
    };

    app.add_systems(Update, mainboard_to_switch_events);
    app.insert_resource(link);
  }
}

#[derive(Debug, Resource)]
pub struct MainboardLink {
  command_tx: mpsc::Sender<MainboardCommand>,
  event_rx: broadcast::Receiver<MainboardIncoming>,
}

impl MainboardLink {
  fn send(&mut self, command: MainboardCommand) {
    self.command_tx.try_send(command).unwrap();
  }

  pub fn enable_watchdog(&mut self) {
    self.send(MainboardCommand::Watchdog(true));
  }
}

fn mainboard_to_switch_events(
  mut mainboard: ResMut<MainboardLink>,
  mut commands: Commands,
  network: Res<IoNetwork>,
) {
  while let Ok(incoming) = mainboard.event_rx.try_recv() {
    match incoming.data {
      FastResponse::SwitchOpened { switch_id } => {
        // TODO
        // if let Some(entity) = network.get_switch_entity(switch_id) {
        //   commands.trigger_targets(SwitchChanged { state }, entity);
        // }
      }
      _ => {}
    }
  }
}
