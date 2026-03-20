use crate::layer::Layer;

pub struct Frame {
  pub width: usize,
  pub height: usize,
  pub layers: Vec<Layer>,
}

impl Frame {
  pub fn new(width: usize, height: usize, layer_count: usize) -> Self {
    let layers: Vec<Layer> = (0..layer_count)
      .map(|_| Layer::new(width, height))
      .collect();

    Self {
      width,
      height,
      layers,
    }
  }

  /// Flatten out frame into pixels for sending to the DMD
  pub fn to_pixels(&self) -> Vec<u8> {
    let mut pixels = vec![0u8; self.width * self.height * 3];
    for layer in &self.layers {
      for y in 0..layer.height {
        for x in 0..layer.width {
          if layer.mask[y * layer.width + x] {
            let idx = (y * layer.width + x) * 3;
            pixels[idx] = layer.pixels[idx];
            pixels[idx + 1] = layer.pixels[idx + 1];
            pixels[idx + 2] = layer.pixels[idx + 2];
          }
        }
      }
    }
    pixels
  }
}
