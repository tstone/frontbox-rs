mod modes;
mod game;
mod hardware;
mod machine;
mod mainboard;
mod modes_old;

// --- protocol ----

#[cfg(feature = "protocol")]
pub mod protocol;

// --- mainboard ---

#[cfg(feature = "mainboard")]
pub use crate::mainboard::*;

pub mod prelude {
  #[cfg(feature = "mainboard")]
  pub use crate::mainboard::mainboard::*;
  #[cfg(feature = "mainboard")]
  pub use crate::mainboard::*;
}
