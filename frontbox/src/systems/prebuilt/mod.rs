mod action_button_eject;
mod autoplunger;
mod trough;
mod watchdog;

pub use action_button_eject::*;
pub use autoplunger::*;
pub use trough::*;
pub use watchdog::*;

/// Enables basic machine operation. SKip this only if you're implementing custom handling.
pub fn operational() -> Vec<Box<dyn System>> {
  vec![Watchdog::new()]
}
