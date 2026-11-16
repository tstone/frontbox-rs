use std::path::Path;

use frontbox_core::prelude::SystemContext;

use crate::*;

pub trait SoundSystemExt {
  fn preload_sound(&self, name: impl Into<SoundKey>, path: impl AsRef<Path>);
  fn play_sfx(&self, key: impl Into<SoundKey>);
  fn play_callout(&self, request: impl Into<CalloutRequest>);
  /// See [`SoundSystem::callout_pending`]
  fn callout_pending(&self) -> bool;
}

impl<'a> SoundSystemExt for SystemContext<'a> {
  fn preload_sound(&self, name: impl Into<SoundKey>, path: impl AsRef<Path>) {
    with_snd_system(self, |snd_system| {
      snd_system.preload(name, path);
    });
  }

  fn play_sfx(&self, key: impl Into<SoundKey>) {
    with_snd_system(self, |snd_system| {
      snd_system.play_sfx(key);
    });
  }

  fn play_callout(&self, request: impl Into<CalloutRequest>) {
    with_snd_system(self, |snd_system| {
      snd_system.play_callout(request);
    });
  }

  fn callout_pending(&self) -> bool {
    self
      .systems
      .get::<SoundSystem>()
      .is_some_and(|system| system.callout_pending())
  }
}

fn with_snd_system<T>(ctx: &SystemContext, f: impl FnOnce(&mut SoundSystem) -> T) {
  if let Some(mut system) = ctx.systems.get::<SoundSystem>() {
    f(&mut system);
  } else {
    log::error!(target: "frontbox::sound", "SoundSystem not running.");
  }
}
