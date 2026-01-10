use env_logger;
use fast_pinball_rs::{Mainboard, Neutron, NeutronConfig};
use log::{error, info};
use std::io;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize logger
  env_logger::init();

  info!("Starting serial port test utility");

  // List all available serial ports
  println!("Available serial ports:");
  match serialport::available_ports() {
    Ok(ports) => {
      for port in &ports {
        println!("  {} - {:?}", port.port_name, port.port_type);
      }

      if ports.is_empty() {
        println!("  No serial ports found");
        return Ok(());
      }
    }
    Err(e) => {
      error!("Error listing serial ports: {}", e);
      return Err(e.into());
    }
  }

  println!("\nEnter the IO/NET serial port name (or press Enter to use the first available):");
  let mut input = String::new();
  io::stdin().read_line(&mut input)?;
  let io_net_port_name: &'static str = Box::leak(input.trim().to_string().into_boxed_str());

  // println!("\nEnter the EXP serial port name (or press Enter to use the first available):");
  // let mut input = String::new();
  // io::stdin().read_line(&mut input)?;
  // let exp_port_name: &'static str = Box::leak(input.trim().to_string().into_boxed_str());

  println!(
    "Initializing Neutron mainboard on port: '{}'",
    io_net_port_name
  );
  let mut neutron = Neutron::define(NeutronConfig {
    io_net_port_path: io_net_port_name,
    exp_port_path: "",
  });

  neutron.initialize();

  println!("Serial port initialization complete.");
  Ok(())
}
