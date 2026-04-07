#[derive(Debug, Clone)]
pub enum FastPlatform {
  Neuron = 2000,
  RetroSystem11 = 11,
  RetroWPC89 = 89,
  RetroWPC95 = 95,
}

impl FastPlatform {
  pub fn from_name(s: &str) -> Option<Self> {
    let s = s.to_lowercase();
    if s.contains("fp-cpu-2000") {
      Some(FastPlatform::Neuron)
    } else if s.contains("fp-cpu-11") {
      Some(FastPlatform::RetroSystem11)
    } else if s.contains("fp-cpu-89") {
      Some(FastPlatform::RetroWPC89)
    } else if s.contains("fp-cpu-95") {
      Some(FastPlatform::RetroWPC95)
    } else {
      None
    }
  }
}
