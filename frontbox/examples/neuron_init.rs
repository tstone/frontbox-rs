use frontbox::prelude::*;
use std::io::Write;

#[tokio::main]
async fn main() {
  env_logger::Builder::from_default_env()
    .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
    .init();

  App::boot(
    "/dev/ttyACM0",
    "/dev/ttyACM1",
    IoNetworkBuilder::new().build(),
    vec![],
  )
  .await
  .run()
  .await;
}
