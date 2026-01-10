extern crate tokio_pinball;

use tokio_pinball::{Mainboard, MainboardConfig};

#[tokio::main]
async fn main() {
  env_logger::init();
  Mainboard::new(MainboardConfig::default()).run().await;
}
