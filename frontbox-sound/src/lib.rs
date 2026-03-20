mod sound_system;

pub struct PlayMusic(&'static str);
pub struct PlayCallout(&'static str);
pub struct PlaySFX(&'static str);

pub use sound_system::*;
