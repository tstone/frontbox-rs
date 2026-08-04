mod app;
pub mod app_message;
mod boot_config;
mod event;
pub mod run_loop;

use std::collections::HashMap;

pub use app::*;
pub use boot_config::*;
pub use event::*;

use crate::prelude::SystemGroup;

pub const ROOT_GROUP: &'static str = "__root";
pub type Groups = HashMap<&'static str, SystemGroup>;
