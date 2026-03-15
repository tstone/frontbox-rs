mod free_play;
mod trough;
mod watchdog;

pub use free_play::*;
pub use trough::*;
pub use watchdog::*;

/// Enables basic machine operation. SKip this only if you're implementing custom handling.
pub fn operational() -> Vec<Box<dyn System>> {
  vec![Watchdog::new()]
}
