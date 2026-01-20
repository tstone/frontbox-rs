use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::*;
use bevy_ecs::world::DeferredWorld;

use super::MainboardCommand;
use crate::mainboard::mainboard_plugin::Mainboard;

#[derive(Debug, Component)]
#[component(on_insert = on_watchdog_inserted)]
pub struct Watchdog {
  enabled: bool,
}

fn on_watchdog_inserted(world: DeferredWorld, context: HookContext) {
  let watchdog = world.get::<Watchdog>(context.entity).unwrap();

  let mainboard = world.get_resource::<Mainboard>().unwrap();
  mainboard.enable_watchdog();
}
