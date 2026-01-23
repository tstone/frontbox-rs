use std::collections::HashMap;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tokio::sync::{broadcast, mpsc};

use crate::mainboard_io::{MainboardCommand, MainboardIncoming};
use crate::prelude::*;
use crate::protocol::FastResponse;

pub struct Frontbox {
  pub mainboard: Mainboard,
  pub io_network: IoNetworkResources,
}

impl Plugin for Frontbox {
  fn build(&self, app: &mut App) {
    // Mainboard link
    let link = MainboardLink {
      command_tx: self.mainboard.tx(),
      event_rx: self.mainboard.subscribe(),
    };
    app.add_systems(Update, mainboard_to_switch_events);
    app.insert_resource(link);

    // Insert Switches
    let mut switches_by_name = HashMap::new();
    let mut switches_by_id = HashMap::new();

    for switch in self.io_network.switches.iter() {
      let entity = app.world_mut().spawn(switch.clone()).id();
      switches_by_name.insert(switch.name, entity);
      switches_by_id.insert(switch.id as usize, entity);
    }

    app.insert_resource(SwitchCatalog {
      by_name: switches_by_name,
      by_id: switches_by_id,
    });

    // Insert Driver Pins
    let mut driver_pins_by_name = HashMap::new();
    let mut driver_pins_by_id = HashMap::new();

    for driver_pin in self.io_network.driver_pins.iter() {
      let entity = app.world_mut().spawn(driver_pin.clone()).id();
      driver_pins_by_name.insert(driver_pin.name, entity);
      driver_pins_by_id.insert(driver_pin.id as usize, entity);
    }

    app.insert_resource(DriverPinCatalog {
      by_name: driver_pins_by_name,
      by_id: driver_pins_by_id,
    });
  }
}

#[derive(Debug, EntityEvent)]
pub struct SwitchStateChanged {
  pub entity: Entity,
  pub state: SwitchState,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum SwitchState {
  Open,
  Closed,
}

#[derive(Debug, Resource)]
pub struct SwitchCatalog {
  pub by_name: HashMap<&'static str, Entity>,
  pub by_id: HashMap<usize, Entity>,
}

#[derive(Debug, Resource)]
pub struct DriverPinCatalog {
  pub by_name: HashMap<&'static str, Entity>,
  pub by_id: HashMap<usize, Entity>,
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

pub fn mainboard_to_switch_events(
  mut mainboard: ResMut<MainboardLink>,
  mut commands: Commands,
  catalog: Res<SwitchCatalog>,
) {
  while let Ok(incoming) = mainboard.event_rx.try_recv() {
    match incoming.data {
      FastResponse::SwitchOpened { switch_id } => {
        if let Some(entity) = catalog.by_id.get(&switch_id) {
          commands.entity(*entity).trigger(|_| SwitchStateChanged {
            entity: *entity,
            state: SwitchState::Open,
          });
        }
      }
      FastResponse::SwitchClosed { switch_id } => {
        if let Some(entity) = catalog.by_id.get(&switch_id) {
          commands.entity(*entity).trigger(|_| SwitchStateChanged {
            entity: *entity,
            state: SwitchState::Closed,
          });
        }
      }
      _ => {}
    }
  }
}
