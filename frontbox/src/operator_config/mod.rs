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
//!           "Coil Power", // label
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
//! Systems can also declare operator config values independent of hardware (e.g. ball count, max extra balls, etc.)
//! as statics.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # use std::sync::LazyLock;
//! pub static MAX_EXTRA_BALLS: LazyLock<Config<u8>> = LazyLock::new(|| {
//!   Config::new(
//!     "Max Extra Balls", // label
//!     "The most extra balls a player can have per game", // description
//!     5, // default
//!     Ranges::u8(0, 10),
//!   )
//! });
//! ```
//!
//! There is no registration step. The first time a config is read or written, its value is loaded from the saved
//! config file if present and within its domain, otherwise it starts at its default.
//!
//! ### Keys
//!
//! A config's value is stored and saved under its key, which is its label unless set with `.key(...)`. An explicit
//! key is needed when two configs share a label, such as each flipper's "Initial Power" shown under its own menu
//! section, or to keep a saved value when a label is reworded.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! let power = HardwareValue::config("Initial Power", "", Power::FULL, Ranges::full_power())
//!   .key("Left Flipper Initial Power");
//! ```
//!
//! - Hardware config keys must be unique. Every driver's configs are checked at boot, and a key used twice panics,
//!   even when both definitions are identical, since a copy-pasted driver config would otherwise share one value.
//! - Configs declared as statics on the same key share one value when their type, domain and default match. A
//!   mismatch panics on first use in debug builds; in release it's logged and the later definition keeps its own
//!   value, starting at its default.
//!
//! ### Reading Operator Configs
//! 
//! ```rust
//! # use frontbox::prelude::*;
//! # use std::sync::LazyLock;
//! # static MAX_EXTRA_BALLS: LazyLock<Config<u8>> =
//! #   LazyLock::new(|| Config::new("Max Extra Balls", "", 5, Ranges::u8(0, 10)));
//! # fn example(ctx: &SystemContext) {
//! // The value is always present since a default is provided
//! let value: u8 = ctx.config.get(&MAX_EXTRA_BALLS);
//! # }
//! ```

mod choices;
mod config;
mod config_definition;
mod config_domain;
mod config_type;
mod hardware_value;
mod config_collection;
mod range;

pub use choices::*;
pub use config::*;
pub use config_definition::*;
pub use config_domain::*;
pub use config_type::*;
pub use hardware_value::*;
pub use config_collection::*;
pub use range::*;
