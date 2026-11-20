mod bundled_fonts;
mod bundled_scenes;
mod display_definition;
mod display_device;
mod display_manager;
mod display_system;
mod initials_entry_system;
pub mod menu;

pub use bundled_fonts::*;
pub use bundled_scenes::*;
pub use display_definition::*;
pub use display_device::*;
pub use display_system::*;
pub use initials_entry_system::*;

/// What an app with a display uses: the system and its displays, the content scenes open from, and the scene prelude
pub mod prelude {
  pub use crate::{
    DisplayDefinition, DisplaySystem, InitialsEntered, InitialsEntrySystem, content_with_bundled,
  };
  pub use frontbox_scene::prelude::*;
}
