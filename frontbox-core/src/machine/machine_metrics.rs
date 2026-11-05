//! Metrics `MachineSystem` records about the machine as a whole. Per-driver fires and per-switch hits are built from
//! the hardware definitions at boot.

use std::sync::LazyLock;

use crate::metrics::Counter;

pub static BOOTS: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Boots", "Times the app started")
    .key("boots")
    .category("Machine")
});

pub static MINUTES_ON: LazyLock<Counter> = LazyLock::new(|| {
  Counter::new("Minutes On", "Minutes the app has been running")
    .key("minutes_on")
    .category("Machine")
});
