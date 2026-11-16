use image::Rgb;

/// A display that frames can be written to, such as `frontbox-dmd`'s `Pin2Dmd`. Its methods are called from a thread
/// of its own, so they can block.
pub trait DisplayDevice: Send + 'static {
  /// The display's size in pixels. Every frame written has exactly this many.
  fn size(&self) -> DisplaySize;

  /// Find and open the device. Called again after a failure, so it should replace anything already open.
  fn open(&mut self) -> Result<(), String>;

  /// Write a frame, row-major from the top left. An error means the device is lost, and it's opened again.
  fn write(&mut self, frame: &[Rgb<u8>]) -> Result<(), String>;
}

/// Something besides the device that shows a display's frames, such as the console. It's given each frame that differs
/// from the last, on the run loop, so it should only keep the frame and leave the work of showing it to another thread.
pub trait DisplayMirror: 'static {
  /// Called once when the [`DisplaySystem`](crate::DisplaySystem) spawns, with the display's name and size
  fn attach(&mut self, display: &str, size: DisplaySize);

  /// A frame of the size given to `attach`, row-major from the top left
  fn show(&mut self, frame: &[Rgb<u8>]);
}

/// A display's size in pixels
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct DisplaySize {
  pub width: u32,
  pub height: u32,
}

impl DisplaySize {
  pub fn new(width: u32, height: u32) -> Self {
    Self { width, height }
  }

  /// How many pixels a frame has
  pub fn pixels(&self) -> usize {
    self.width as usize * self.height as usize
  }
}
