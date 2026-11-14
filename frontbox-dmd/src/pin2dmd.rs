use std::time::Duration;

use image::Rgb;
use rusb::UsbContext;

use frontbox_display::{DisplayDevice, DisplaySize};

const VENDOR_ID: u16 = 0x0314;
const PRODUCT_ID: u16 = 0xE457;
/// A frame takes a few milliseconds to send. Longer than this means the device has stalled.
const WRITE_TIMEOUT: Duration = Duration::from_millis(100);

/// Derived from Mission Pinball Framework PIN2DMD driver
/// https://github.com/missionpinball/mpf
/// MIT License
const GAMMA_TABLE: [u8; 256] = [
  0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
  1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
  2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5,
  5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 10, 10, 10,
  10, 11, 11, 11, 11, 11, 12, 12, 12, 12, 13, 13, 13, 13, 13, 14, 14, 14, 14, 15, 15, 15, 16, 16,
  16, 16, 17, 17, 17, 18, 18, 18, 18, 19, 19, 19, 20, 20, 20, 21, 21, 21, 22, 22, 22, 23, 23, 23,
  24, 24, 24, 25, 25, 25, 26, 26, 27, 27, 27, 28, 28, 29, 29, 29, 30, 30, 31, 31, 31, 32, 32, 33,
  33, 34, 34, 35, 35, 35, 36, 36, 37, 37, 38, 38, 39, 39, 40, 40, 41, 41, 42, 42, 43, 43, 44, 44,
  45, 45, 46, 47, 47, 48, 48, 49, 49, 50, 50, 51, 52, 52, 53, 53, 54, 55, 55, 56, 56, 57, 58, 58,
  59, 60, 60, 61, 62, 62, 63, 63, 63,
];

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum PanelType {
  Rgb,
  Rbg,
}

/// A PIN2DMD display connected over USB
pub struct Pin2Dmd {
  handle: Option<rusb::DeviceHandle<rusb::Context>>,
  panel: PanelType,
  size: DisplaySize,
  elements: usize,
}

impl Pin2Dmd {
  /// The device is opened once [`DisplaySystem`](frontbox_display::DisplaySystem) spawns
  pub fn new(width: u32, height: u32, panel: PanelType) -> Self {
    Self {
      handle: None,
      panel,
      size: DisplaySize::new(width, height),
      elements: width as usize * height as usize / 2,
    }
  }

  /// Pack a frame into the PIN2DMD wire format
  fn pack(&self, pixels: &[Rgb<u8>]) -> Vec<u8> {
    let mut buf = vec![0u8; self.elements * 6 + 4];
    buf[0] = 0x81;
    buf[1] = 0xC3;
    buf[2] = 0xE9;
    buf[3] = 18;

    for i in 0..self.elements {
      // each element carries a pixel from the top half and the one at the same place in the bottom half
      let Rgb([r, g, b]) = pixels[i];
      let Rgb([rl, gl, bl]) = pixels[self.elements + i];

      // Select channel order based on panel type, matching MPF rgb/rbg logic
      let (pr, pg, pb, prl, pgl, pbl) = match self.panel {
        PanelType::Rgb => (r, g, b, rl, gl, bl),
        PanelType::Rbg => (r, b, g, rl, bl, gl),
      };

      // Gamma correction
      let mut pr = GAMMA_TABLE[pr as usize];
      let mut pg = GAMMA_TABLE[pg as usize];
      let mut pb = GAMMA_TABLE[pb as usize];
      let mut prl = GAMMA_TABLE[prl as usize];
      let mut pgl = GAMMA_TABLE[pgl as usize];
      let mut pbl = GAMMA_TABLE[pbl as usize];

      // Write 6 bitplanes, LSB first, interleaving top and bottom half
      let mut target_idx = i + 4;
      for _ in 0..6 {
        buf[target_idx] = ((pgl & 1) << 5)
          | ((pbl & 1) << 4)
          | ((prl & 1) << 3)
          | ((pg & 1) << 2)
          | ((pb & 1) << 1)
          | (pr & 1);

        pr >>= 1;
        pg >>= 1;
        pb >>= 1;
        prl >>= 1;
        pgl >>= 1;
        pbl >>= 1;

        target_idx += self.elements;
      }
    }

    buf
  }
}

impl DisplayDevice for Pin2Dmd {
  fn size(&self) -> DisplaySize {
    self.size
  }

  fn open(&mut self) -> Result<(), String> {
    self.handle = None;
    let context = rusb::Context::new().map_err(|e| e.to_string())?;
    let device = context
      .devices()
      .map_err(|e| format!("couldn't list USB devices: {}", e))?
      .iter()
      .find(|device| {
        device
          .device_descriptor()
          .is_ok_and(|desc| desc.vendor_id() == VENDOR_ID && desc.product_id() == PRODUCT_ID)
      })
      .ok_or("no PIN2DMD connected")?;

    let handle = device.open().map_err(|e| e.to_string())?;
    handle.claim_interface(0).map_err(|e| e.to_string())?;
    self.handle = Some(handle);
    Ok(())
  }

  fn write(&mut self, frame: &[Rgb<u8>]) -> Result<(), String> {
    let expected = self.size.width as usize * self.size.height as usize;
    if frame.len() != expected {
      return Err(format!("a frame of {} pixels, where the display has {}", frame.len(), expected));
    }
    let buf = self.pack(frame);
    let handle = self.handle.as_ref().ok_or("not open")?;
    handle
      .write_bulk(0x01, &buf, WRITE_TIMEOUT)
      .map_err(|e| e.to_string())?;
    Ok(())
  }
}
