use std::path::PathBuf;

use frontbox_core::prelude::*;
use frontbox_scene::PlaybackId;

use crate::SoundKey;
use crate::callout::CalloutRequest;

/// What [`SoundSystem`](crate::SoundSystem) sends the [`SoundManager`](super::SoundManager) to do
pub(crate) enum SoundManagerMessage {
  PreloadPath(SoundKey, PathBuf),
  PreloadEmbedded(SoundKey, &'static [u8]),
  /// A preloaded sound, at a volume where 1.0 is as recorded, from a time into it, and cut off after a time if given.
  /// One a scene's playback plays names it, so [`StopPlaybackSfx`](Self::StopPlaybackSfx) can cut it off.
  PlaySfx(
    SoundKey,
    f32,
    Duration,
    Option<Duration>,
    Option<PlaybackId>,
  ),
  /// Cut off the sounds a playback is still playing
  StopPlaybackSfx(PlaybackId),
  PlayCallout(CalloutRequest),
  SetCalloutVolume(f32),
  PlayMusic(PathBuf, Duration),
  StopMusic(Duration),
}
