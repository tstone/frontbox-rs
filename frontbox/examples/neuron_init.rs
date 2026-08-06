use frontbox::prelude::*;
use std::io::Write;

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  App::boot(BootConfig {
    io_net_port_path: "/dev/ttyACM0",
    exp_port_path: "/dev/ttyACM1",
    io_network: IoNetwork::empty(),
    exp_network: ExpNetwork::empty(),
    system_interval: Duration::from_millis(83),
    watchdog_interval: Duration::from_secs(1),
    ..Default::default()
  })
  .await
  .run()
  .await;
}
