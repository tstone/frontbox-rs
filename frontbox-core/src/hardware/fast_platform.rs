/// The FAST controllers, by the number each is configured with. Only the Neuron is supported.
#[repr(u16)]
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum FastPlatform {
  Neuron = 2000,
  RetroSystem11 = 11,
  RetroWPC89 = 89,
  RetroWPC95 = 95,
}

impl From<FastPlatform> for u16 {
  fn from(platform: FastPlatform) -> Self {
    platform as u16
  }
}
