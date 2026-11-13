use std::path::PathBuf;

use frontbox_core::prelude::*;

use crate::callout::CalloutRequest;

/// What [`SoundSystem`](crate::SoundSystem) sends the [`SoundManager`](super::SoundManager) to do
pub(crate) enum SoundManagerMessage {
  PreloadPath(&'static str, PathBuf),
  PreloadEmbedded(&'static str, &'static [u8]),
  PlaySfx(&'static str),
  PlayCallout(CalloutRequest),
  SetCalloutVolume(f32),
  PlayMusic(PathBuf, Duration),
  StopMusic(Duration),
}
