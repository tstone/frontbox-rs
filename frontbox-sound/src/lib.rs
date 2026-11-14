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
//! - Overlapping requests queue in the order they were asked for, unless a [`CalloutRequest`] says otherwise
//! - Automatically lowers volume on music track when playing
//! - [`CalloutCompleted`] says when a line is done with
//! 
//! ## Scene sounds
//!
//! - A scene's `Sound` layers name sounds in the app's content folder, such as `sounds/zap.mp3`
//! - Loaded the first time a scene plays them, or ahead of time with [`SoundSystem::preload_scene`]
//! - Each plays when the playback's clock passes one of its layer's start times, as an effect would
//!
//! ## Music
//! 
//! - Stream from disk
//! - Can only play one at a time
//! - Overlapping requests overwrite previous track
//! - Can crossfade into each other
//! 
//! ```rust
//! # use frontbox_core::prelude::*;
//! # use frontbox_sound::*;
//! # fn example(ctx: &SystemContext) {
//! let mut sound_system = ctx.systems.expect::<SoundSystem>();
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

mod callout;
mod sound_system;
mod sound_system_ext;
mod sound_manager;

pub use callout::CalloutRequest;
pub use sound_system::*;
pub use sound_system_ext::*;

use std::borrow::Cow;

use frontbox_core::prelude::*;

/// The name a sound is preloaded and played by: a constant in the game's code, or a name from a scene
pub type SoundKey = Cow<'static, str>;

/// A callout is done with, whether it was said or not. It isn't said if a higher priority replaced it, it waited longer
/// than its `max_wait`, or it couldn't be played. So anything waiting on a line, like a mode's intro, always hears back.
#[derive(serde::Serialize, Event, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalloutCompleted {
  pub key: &'static str,
  pub played: bool,
}
