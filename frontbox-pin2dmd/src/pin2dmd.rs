use frontbox_canvas::Size;
use rusb::UsbContext;
use std::time::Duration;

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
pub struct Pin2Dmd {
  handle: rusb::DeviceHandle<rusb::Context>,
  panel: PanelType,
  pub size: Size<u32>,
  elements: usize,
}

impl Pin2Dmd {
  pub fn connect(width: u32, height: u32, panel: PanelType) -> rusb::Result<Self> {
    let context = rusb::Context::new()?;
    let device = context
      .devices()?
      .iter()
      .find(|d| {
        let desc = d.device_descriptor().unwrap();
        desc.vendor_id() == 0x0314 && desc.product_id() == 0xE457
      })
      .expect("PIN2DMD not found");

    let handle = device.open()?;
    handle.claim_interface(0)?;
    Ok(Self {
      handle,
      panel,
      size: Size::new(width, height),
      elements: width as usize * height as usize / 2,
    })
  }

  /// `pixels` is WIDTH*HEIGHT*3 bytes, RGB order, row-major, top-left to bottom-right.
  pub fn render(&mut self, pixels: &[u8]) -> rusb::Result<()> {
    let buf = self.pack_rgb24(pixels, self.panel);
    self
      .handle
      .write_bulk(0x01, &buf, Duration::from_millis(2000))?;
    Ok(())
  }

  pub fn clear(&mut self) -> rusb::Result<()> {
    let pixels = vec![0u8; self.size.width as usize * self.size.height as usize * 3];
    self.render(&pixels)
  }

  /// pack an RGB24 frame into the PIN2DMD wire format
  pub fn pack_rgb24(&self, pixels: &[u8], panel: PanelType) -> Vec<u8> {
    let expected_len = self.size.width * self.size.height * 3;
    assert_eq!(
      pixels.len() as u32,
      expected_len,
      "Expected {} pixels to render PIN2DMD but received {}",
      expected_len,
      pixels.len()
    );
    let mut buf = vec![0u8; self.elements * 6 + 4];
    buf[0] = 0x81;
    buf[1] = 0xC3;
    buf[2] = 0xE9;
    buf[3] = 18;

    for i in 0..self.elements {
      let idx = i * 3;

      // Select channel order based on panel type, matching MPF rgb/rbg logic
      let (pr, pg, pb, prl, pgl, pbl) = match panel {
        PanelType::Rgb => (
          pixels[idx],
          pixels[idx + 1],
          pixels[idx + 2],
          pixels[self.elements * 3 + idx],
          pixels[self.elements * 3 + idx + 1],
          pixels[self.elements * 3 + idx + 2],
        ),
        PanelType::Rbg => (
          pixels[idx],
          pixels[idx + 2],
          pixels[idx + 1],
          pixels[self.elements * 3 + idx],
          pixels[self.elements * 3 + idx + 2],
          pixels[self.elements * 3 + idx + 1],
        ),
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
