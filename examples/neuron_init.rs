use bevy_app::{ScheduleRunnerPlugin, prelude::*};
use bevy_ecs::prelude::*;
use frontbox::mainboard_comms::MainboardIncoming;
use frontbox::protocol::configure_hardware::SwitchReporting;
use frontbox::{define_io_network, prelude::*};
use std::time::Duration;

fn main() {
  env_logger::init();

  let io_network = define_io_network! {
    cabinet: FastIoBoards::cabinet() => {
      switches {
        0: start_button,
        1: left_flipper_button,
        2: right_flipper_button,
      }
      drivers {
        0: start_button_light,
      }
    },
    playfield: FastIoBoards::io_3208() => {
      switches {
        0: left_outlane,
        1: left_inlane,
        2: left_ramp_enter,
        3: left_ramp_exit,
        4: right_outlane,
        5: right_inlane,
        6: right_ramp_enter,
        7: right_ramp_exit,
      }
      drivers {
        0: left_flipper,
        1: right_flipper,
        2: popper_1,
        3: popper_2,
      }
    }
  };

  App::new()
    .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))
    .add_plugins(Frontbox {
      mainboard_config: MainboardConfig {
        platform: FastPlatform::Neuron,
        io_net_port_path: "/dev/ttyACM0",
        exp_port_path: "/dev/ttyACM1",
        switch_reporting: Some(SwitchReporting::Verbose),
      },
      io_network,
    })
    .add_systems(Startup, startup)
    .add_observer(on_mainboard_event)
    .run();
}

fn startup(mut mainboard: ResMut<Mainboard>) {
  log::info!("😀 Neuron init example started");
  mainboard.enable_watchdog();
}

// example of listening to raw events from the Neuron
fn on_mainboard_event(event: On<MainboardIncoming>) {
  log::info!("📧 Received mainboard event: {:?}", event);
}
