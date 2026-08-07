//! # Operator Config
//! 
//! Operator config provides a standard way to read operator-level settings and provide structure to build a menu. Operator configs mainly show up in two places: (1) when declaring hardware properties (e.g. the power of a coil) and (2) for use by systems.
//! 
//! ### Configurable Hardware Values
//! 
//! Many hardware settings actually require a `HardwareValue`. For static values that remain for the life of the program, `HardwareValue::fixed` supports this. But for values that can be configured by the operator config, `HardwareValue::config` will make the value configurable.
//! 
//! ```rust
//! hardware_defs! {
//!   pub MY_COIL: DriverDefinition = DriverDefinition::new("my_coil")
//!     .mode(PulseMode {
//!       trigger_mode: DriverTriggerMode::VirtualSwitchTrue,
//!       // static value for the life of the program
//!       initial_pwm_length: HardwareValue::fixed(Duration::from_millis(250)),
//!       // configurable value that can be adjusted via operator config
//!       initial_pwm_power: HardwareValue::config(
//!         "coil_power",  // name
//!         Power::THREE_QUARTERS // default
//!         Ranges::power(0.5, 1.0), // domain
//!       ),
//!       ..Default::default()
//!     });
//! }
//! ```
//! 
//! ### System Config Values
//! 
//! Systems can also register operator config values independent of hardware (e.g. ball count, max extra balls, etc.). This is done through the `config_values` method of `System`.
//! 
//! ```rust
//! pub static MAX_EXTRA_BALLS: LazyLock<ConfigValue<u8, Range<u8>>> = LazyLock::new(|| {
//!   ConfigValue::new(
//!     "Max Extra Balls", // name
//!     "The most extra balls a player can have per game", // description
//!     5, // default
//!     Ranges::u8(0, 10),
//!   )
//! });
//! 
//! impl System for Example {
//!   fn config_values(&self) -> Vec<&'static dyn LoadableConfigValue> {
//!     vec![&*MAX_EXTRA_BALLS]
//!   }
//! }
//! ```
//! 
//! ### System Config Registration
//! 
//! - Startup systems have their config values automatically registered
//! - Dynamically loaded systems must be manually registered
//! 
//! ```rust
//!   App::boot(BootConfig::default()).await
//!     .configure(|app| {
//!       // config values will be automatically registered
//!       app.system(MySystem::new())
//! 
//!       // manually register configs on a system
//!       app.register_configs(some_system)
//! 
//!       // or manually register explicit configs
//!       app.register_configs(vec![MY_CONFIG1, MY_CONFIG2])
//!     })
//! ```
//! 
//! ### Reading Operator Configs
//! 
//! ```rust
//! // The value is always present since a default is provided
//! let value: u8 = ctx.operator_config.get(MAX_EXTRA_BALLS);
//! ```

mod config_display;
mod config_value;
mod domain;
mod generalized_config_value;
mod hardware_value;
mod operator_config;
mod range;

pub use config_display::*;
pub use config_value::*;
pub use domain::*;
pub use generalized_config_value::*;
pub use hardware_value::*;
pub use operator_config::*;
pub use range::*;
