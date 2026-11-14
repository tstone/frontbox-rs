use frontbox_gfx::canvas::Size;
use image::Rgb;

/// A dot matrix display that frames can be written to, such as a [`Pin2Dmd`](crate::Pin2Dmd). Its methods are called
/// from a thread of its own, so they can block.
pub trait DmdDevice: Send + 'static {
  /// The display's size in pixels. Every frame written has exactly this many.
  fn size(&self) -> Size<u32>;

  /// Find and open the device. Called again after a failure, so it should replace anything already open.
  fn open(&mut self) -> Result<(), String>;

  /// Write a frame, row-major from the top left. An error means the device is lost, and it's opened again.
  fn write(&mut self, frame: &[Rgb<u8>]) -> Result<(), String>;
}
