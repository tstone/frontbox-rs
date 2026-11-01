//! # App
//!
//! <div class="warning">Stability Level: Moderate</div>
//!
//! App is the runnable root of a Frontbox project. Every machine runs exactly one app. Apps provide a place to
//! specify boot configuration, immutable settings (COM ports, hardware, etc.), and initial systems.
//!
//! An app has two distinct phases:
//!   1. **Configuration** - Defining initial systems and tracers
//!   2. **Boot & Run** -  The main event loop processes events and systems
//!
//! - See [BootConfig] for details on what is configurable.
//! - See [mod@crate::hardware] for details on setting up I/O and expansion networks.
//! - See [mod@crate::operator_config] for details on operator configuration.
//!
//! Running an  `App` is an async process which requires a [Tokio](https://tokio.rs/) runtime. This is
//! most easily achieved by tagging the main function as `#[tokio::main]`.
//!
//! # Examples
//!
//! ```rust,no_run
//! use frontbox::prelude::*;
//! use std::io::Write;
//! # struct MySystem;
//! # impl MySystem { fn new() -> Self { Self } }
//! # impl System for MySystem {}
//! # struct MySystem2;
//! # impl MySystem2 { fn new() -> Self { Self } }
//! # impl System for MySystem2 {}
//!
//! #[tokio::main]
//! async fn main() {
//!   // Frontbox emits log events but requires a configured logger to print them out
//!   env_logger::Builder::from_default_env()
//!     .format(|buf, record| writeln!(buf, "[{}] {}\r", record.level(), record.args()))
//!     .init();
//!
//!   App::new(BootConfig {
//!       platform: Platform::Neuron {
//!         io_net_port_path: "/dev/ttyACM0",
//!         exp_port_path: "/dev/ttyACM1",
//!         watchdog_interval: Duration::from_millis(1500),
//!       },
//!       // see section on hardware for how these are configured
//!       io_network: IoNetwork::empty(),
//!       ..Default::default()
//!     })
//!     .configure(|app| {
//!       // add initial system(s) that will start on `.run()`
//!       app.system(MySystem::new());
//!       app.system(MySystem2::new());
//!     })
//!     // Running the app boots the hardware and starts the game
//!     .run()
//!     .await;
//! }
//! ```

mod app;
pub(crate) mod app_config;
pub(crate) mod app_message;
mod app_state;
pub mod app_tracer;
mod boot_config;
pub mod event_box;
pub(crate) mod run_loop;

use std::collections::HashMap;

pub use app::*;
pub(crate) use app_config::*;
pub use app_message::ShutdownScope;
pub use app_state::*;
pub use boot_config::*;
pub(crate) use event_box::*;
pub use run_loop::Synchronize;

use crate::prelude::SystemGroup;

pub const ROOT_GROUP: &'static str = "__root";
pub(crate) type Groups = HashMap<&'static str, SystemGroup>;
