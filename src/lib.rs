mod hardware;
mod mainboard;

// --- protocol ----

#[cfg(feature = "protocol")]
pub mod protocol;

// --- mainboard ---

#[cfg(feature = "mainboard")]
pub use mainboard::*;
