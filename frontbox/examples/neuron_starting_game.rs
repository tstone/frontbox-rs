use frontbox::plugins::free_play::FreePlay;
use frontbox::plugins::player_system::PlayerSystem;
use frontbox::prelude::*;
use std::io::Write;

pub mod switches {
  pub const START_BUTTON: &str = "start_button";
}

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  MachineBuilder::boot(
    BootConfig {
      platform: FastPlatform::Neuron,
      io_net_port_path: "/dev/ttyACM0",
      exp_port_path: "/dev/ttyACM1",
      ..Default::default()
    },
    IoNetworkBuilder::new().build(),
    vec![],
  )
  .await
  .add_virtual_switch(KeyCode::Home, switches::START_BUTTON)
  .build()
  .run(vec![
    FreePlay::new(switches::START_BUTTON),
    // This system listens for game start and spawns up a new player district. In a real machine the game type
    // be it players, co-op, team may be selectable. This little bit of glue code is responsible for translating
    // from game type to what is actually running.
    OnEventSystem::<GameStarted>::new(|_ctx, cmds| {
      cmds.system.spawn(*PlayerSystem::new(vec![]));
    }),
  ])
  .await;
}
