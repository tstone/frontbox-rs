use frontbox::prelude::*;
use frontbox::districts::AttractMode;
use std::io::Write;

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
    IoNetworkSpec::new().build(),
    vec![],
  )
  .await
  .build()
  .run(AttractMode::new(vec![]))
  .await;
}
