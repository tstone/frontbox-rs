use crate::hardware::boards::Pin;
use crate::hardware::driver_config::DriverConfig;

/// A "device" represents hardware that uses one or more driver pins
pub enum Device {
  SingleCoil {
    driver_pin: Pin,
    config: DriverConfig,
  },
  DoubleCoil {
    main_driver_pin: Pin,
    aux_driver_pin: Pin,
    main_config: DriverConfig,
    aux_config: DriverConfig,
  },
}
