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
//! let sound_system = ctx.expect::<SoundSystem>();
//! 
//! // typically done `on_startup`
//! sound_system.preload("name", "/game/assets/sfx/example.wav");
//! sound_system.preload("multiball", "/game/assets/callouts/multiball.wav");
//! 
//! sound_system.play_sfx("name");
//! sound_system.play_callout("multiball");
//! sound_system.play_music("/game/assets/music/track1.mp3");
//! sound_system.crossfade_music("/game/assets/music/track2.mp3");
//! ```

mod sound_system;
mod sound_system_ext;

pub use sound_system::*;
pub use sound_system_ext::*;
