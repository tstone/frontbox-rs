use std::time::Duration;

use frontbox::prelude::*;
use frontbox_pin2dmd::menu::*;

fn main() {
  hardware_defs! {
    pub DROP_COIL: DriverDefinition = DriverDefinition::new("example")
      .mode(PulseMode {
        initial_pwm_length: HardwareValue::config(
          "Drop Target Reset Duration",
          "Amount of time fire the coil to reset the bank",
          Duration::from_millis(35),
          Ranges::duration(5, 100),
        ),
        ..Default::default()
      });
  }

  let _menu = MenuSection::root()
    .section(MenuSection::new("Section 1").configs(DROP_COIL.generalized_config_values()))
    .section(MenuSection::new("Section 2"));
}
