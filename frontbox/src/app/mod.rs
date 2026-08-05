//! App is the runnable root of a Frontbox project. Every machine runs exactly one app. Apps provide a place to specify boot configuration, immutable settings (COM ports, hardware, etc.), and initial systems.
//! 
//! App is an async process which requires a [Tokio](https://tokio.rs/) runtime. This is most easily achieved by tagging the main function as `#[tokio::main]`.
//! 
//! An app has two distinct phases: (1) booting, where the mainboard and key hardware is initialized; and (2) running, where the main event loop processes events and systems.
//! 
//! # Example
//! 
//! ```rust,no_run
//! use frontbox::prelude::*;
//! use std::io::Write;
//! 
//! #[tokio::main]
//! async fn main() {
//!   // Frontbox emits log events but requires a configured logger to print them out
//!   env_logger::Builder::from_default_env()
//!     .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
//!     .init();
//!   
//!   // Booting the app initializes hardware
//!   App::boot(BootConfig {
//!     io_net_port_path: "/dev/ttyACM0",
//!     // see section on hardware for how these are configured
//!     io_network: IoNetwork::empty(),
//!     ..Default::default()
//!   })
//!   .await
//!   // Running the app starts the game
//!   .run()
//!   .await;
//! }
//! ```

mod app;
pub(crate) mod app_message;
mod boot_config;
mod event;
mod into_configs;
pub(crate) mod run_loop;

use std::collections::HashMap;

pub(crate) use into_configs::*;
pub use app::*;
pub use boot_config::*;
pub use event::*;

use crate::prelude::SystemGroup;

pub const ROOT_GROUP: &'static str = "__root";
pub(crate) type Groups = HashMap<&'static str, SystemGroup>;
