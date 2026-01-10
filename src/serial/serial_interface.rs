use serialport::SerialPort;
use std::thread;
use std::time::Duration;

use crate::protocol::FastResponse;
use crate::serial::line_parser::LINE_PARSER;
use crate::serial::parser::SerialParser;

const BAUD_RATE: u32 = 921_600;

pub struct SerialInterface {
  port: Box<dyn SerialPort>,
  buffer: Vec<u8>,
  parser: SerialParser,
}

/// SerialInterface joins a port to a parser. This is necessary because the Fast protocol does not
/// return consistent responses, depending on the input command. Usually it's a one-liner but sometimes
/// it contains binary data and other times it's a table. The different ways of "read next" are wrapped
/// up in a SerialParser.
impl SerialInterface {
  pub fn open(port_path: &str) -> Self {
    let port = serialport::new(port_path, BAUD_RATE)
      .parity(serialport::Parity::None)
      .stop_bits(serialport::StopBits::One)
      .flow_control(serialport::FlowControl::None)
      .open();

    match port {
      Ok(port) => Self {
        port,
        buffer: Vec::new(),
        parser: LINE_PARSER,
      },
      Err(e) => {
        log::error!("{:?} - {}", e.kind, e.description);
        thread::sleep(Duration::from_millis(300));
        return Self::open(port_path);
      }
    }
  }

  pub fn send(&mut self, data: &[u8]) {
    match self.port.write_all(data) {
      Ok(_) => {}
      Err(e) => {
        log::error!("Error sending msg: {}", e);
      }
    }
  }

  pub fn send_str(&mut self, data: &str) {
    self.send(data.as_bytes());
  }

  pub fn read_next(&mut self) -> Option<FastResponse> {
    let _ = self.port.read_to_end(&mut self.buffer).unwrap_or_else(|e| {
      log::error!("Error reading from serial port: {}", e);
      0
    });
    (self.parser)(&mut self.buffer)
  }
}
