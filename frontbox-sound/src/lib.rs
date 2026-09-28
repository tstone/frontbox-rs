//! # Frontbox Sound
//! 
//! <div class="warning">Stability Level: Low</div>
//! 
//! Frontbox includes `SoundSystem` that supports three types of sounds:
//! 
//! ## Effects
//! 
//! - Must be preloaded
//! - Can play unlimited at a time
//! 
//! ## Callouts
//! 
//! - Must be preloaded
//! - Can play one at a time
//! - Overlapping requests will queue
//! - Automatically lowers volume on music track when playing
//! 
//! ## Music
//! 
//! - Stream from disk
//! - Can only play one at a time
//! - Overlapping requests overwrite previous track
//! - Can crossfade into each other
//! 
//! ```rust
//! # use frontbox::prelude::*;
//! # use frontbox_sound::*;
//! # fn example(ctx: &SystemContext) {
//! let mut sound_system = ctx.expect::<SoundSystem>();
//!
//! // typically done `on_spawn`
//! sound_system.preload("name", "/game/assets/sfx/example.wav");
//! sound_system.preload("multiball", "/game/assets/callouts/multiball.wav");
//!
//! sound_system.play_sfx("name");
//! sound_system.play_callout("multiball");
//! sound_system.play_music("/game/assets/music/track1.mp3", Duration::ZERO);
//! // crossfade into the next track
//! sound_system.play_music("/game/assets/music/track2.mp3", Duration::from_secs(2));
//! # }
//! ```

mod sound_system;
mod sound_system_ext;
mod sound_manager;

pub use sound_system::*;
pub use sound_system_ext::*;
