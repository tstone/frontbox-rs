use serialport::SerialPort;

use crate::mainboard::mainboard::Mainboard;

const BAUD_RATE: u32 = 921_600;

pub struct Neutron {
    config: NeutronConfig,
    io_net_port: Option<Box<dyn SerialPort>>,
    exp_port: Option<Box<dyn SerialPort>>,
}

pub struct NeutronConfig {
    pub io_net_port_path: &'static str,
    pub exp_port_path: &'static str,
}

impl Neutron {
    pub fn define(config: NeutronConfig) -> Self {
        Self {
            config,
            io_net_port: None,
            exp_port: None,
        }
    }

    fn init_io_net_port(&mut self) {
        match serialport::new(self.config.io_net_port_path, BAUD_RATE)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .open()
        {
            Ok(port) => {
                self.io_net_port = Some(port);
                log::info!(
                    "IO/NET port initialized at {}",
                    self.config.io_net_port_path
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to open IO/NET port {}: {:?}",
                    self.config.io_net_port_path,
                    e
                );
            }
        }
    }

    fn init_exp_port(&mut self) {
        match serialport::new(self.config.exp_port_path, BAUD_RATE)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .open()
        {
            Ok(port) => {
                self.exp_port = Some(port);
                log::info!("EXP port initialized at {}", self.config.exp_port_path);
            }
            Err(e) => {
                log::error!(
                    "Failed to open EXP port {}: {:?}",
                    self.config.exp_port_path,
                    e
                );
            }
        }
    }
}

impl Mainboard for Neutron {
    fn initialize(&mut self) {
        self.init_io_net_port();
        self.init_exp_port();
    }
}
