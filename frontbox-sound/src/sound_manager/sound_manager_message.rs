use std::path::PathBuf;

use frontbox_core::prelude::*;

use crate::SoundKey;
use crate::callout::CalloutRequest;

/// What [`SoundSystem`](crate::SoundSystem) sends the [`SoundManager`](super::SoundManager) to do
pub(crate) enum SoundManagerMessage {
  PreloadPath(SoundKey, PathBuf),
  PreloadEmbedded(SoundKey, &'static [u8]),
  /// A preloaded sound, at a volume where 1.0 is as recorded, from a time into it, and cut off after a time if given
  PlaySfx(SoundKey, f32, Duration, Option<Duration>),
  PlayCallout(CalloutRequest),
  SetCalloutVolume(f32),
  PlayMusic(PathBuf, Duration),
  StopMusic(Duration),
}
