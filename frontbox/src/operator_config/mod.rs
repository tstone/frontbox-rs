//! # Operator Config
//! 
//! Operator config provides a standard way to read operator-level settings and provide structure to build a menu. Operator configs mainly show up in two places: (1) when declaring hardware properties (e.g. the power of a coil) and (2) for use by systems.
//! 
//! ### Configurable Hardware Values
//! 
//! Many hardware settings actually require a `HardwareValue`. For static values that remain for the life of the program, `HardwareValue::fixed` supports this. But for values that can be configured by the operator config, `HardwareValue::config` will make the value configurable.
//! 
//! ```rust
//! # use frontbox::prelude::*;
//! hardware_defs! {
//!   pub MY_COIL: DriverDefinition = DriverDefinition::new("my_coil")
//!     .mode(
//!       DriverMode::pulse()
//!         .trigger_mode(DriverTriggerMode::VirtualSwitchTrue)
//!         // static value for the life of the program (plain values convert to `HardwareValue::fixed`)
//!         .initial_pwm_length(Duration::from_millis(250))
//!         // configurable value that can be adjusted via operator config
//!         .initial_pwm_power(HardwareValue::config(
//!           "coil_power", // name
//!           "Power of my coil", // description
//!           Power::THREE_QUARTERS, // default
//!           Ranges::power(128, 255), // domain
//!         ))
//!         .build(),
//!     );
//! }
//! ```
//! 
//! ### System Config Values
//! 
//! Systems can also register operator config values independent of hardware (e.g. ball count, max extra balls, etc.). This is done through the `config_values` method of `System`.
//! 
//! ```rust
//! # use frontbox::prelude::*;
//! # use std::sync::LazyLock;
//! # struct Example;
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
//!   fn config_values(&self) -> Vec<&'static dyn GeneralizedConfigValue> {
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
//! # use frontbox::prelude::*;
//! # use std::sync::LazyLock;
//! # struct MySystem;
//! # impl MySystem { fn new() -> Self { Self } }
//! # impl System for MySystem {}
//! # let some_system = MySystem::new();
//! # static MY_CONFIG1: LazyLock<ConfigValue<u8, Range<u8>>> =
//! #   LazyLock::new(|| ConfigValue::new("Config 1", "", 1, Ranges::u8(0, 10)));
//! # static MY_CONFIG2: LazyLock<ConfigValue<u8, Range<u8>>> =
//! #   LazyLock::new(|| ConfigValue::new("Config 2", "", 2, Ranges::u8(0, 10)));
//! App::new(BootConfig::default())
//!   .configure(|app| {
//!     // config values will be automatically registered
//!     app.system(MySystem::new());
//! 
//!     // manually register configs on a system
//!     app.register_configs(&some_system);
//! 
//!     // or manually register explicit configs
//!     let configs: Vec<&'static dyn GeneralizedConfigValue> = vec![&*MY_CONFIG1, &*MY_CONFIG2];
//!     app.register_configs(configs);
//!   });
//! ```
//! 
//! ### Reading Operator Configs
//! 
//! ```rust
//! # use frontbox::prelude::*;
//! # use std::sync::LazyLock;
//! # static MAX_EXTRA_BALLS: LazyLock<ConfigValue<u8, Range<u8>>> =
//! #   LazyLock::new(|| ConfigValue::new("Max Extra Balls", "", 5, Ranges::u8(0, 10)));
//! # fn example(ctx: &SystemContext) {
//! // The value is always present since a default is provided
//! let value: u8 = ctx.operator_config.get(&MAX_EXTRA_BALLS);
//! # }
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
