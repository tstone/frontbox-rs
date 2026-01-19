mod fast_codec;
mod hardware;
mod mainboard;
mod serial_interface;

// --- protocol ----

#[cfg(feature = "protocol")]
pub mod protocol;

// --- mainboard ---

#[cfg(feature = "mainboard")]
pub use mainboard::*;
