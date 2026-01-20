mod frontbox;
mod hardware;
mod mainboard;

// --- protocol ----

#[cfg(feature = "protocol")]
pub mod protocol;

// --- mainboard ---

#[cfg(feature = "mainboard")]
pub use crate::mainboard::*;

pub mod prelude {
  #[cfg(feature = "mainboard")]
  pub use crate::frontbox::Frontbox;
  #[cfg(feature = "mainboard")]
  pub use crate::mainboard::mainboard_comms::*;
  #[cfg(feature = "mainboard")]
  pub use crate::mainboard::*;
}
