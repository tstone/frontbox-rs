use std::path::Path;

use frontbox::prelude::Context;

use crate::*;

pub trait SoundSystemExt {
  fn preload_sound(&self, name: &'static str, path: impl AsRef<Path>);
  fn play_sfx(&self, key: &'static str);
  fn play_callout(&self, key: &'static str);
}

impl<'a> SoundSystemExt for Context<'a> {
  fn preload_sound(&self, name: &'static str, path: impl AsRef<Path>) {
    with_snd_system(self, |snd_system| {
      snd_system.preload(name, path);
    });
  }

  fn play_sfx(&self, key: &'static str) {
    with_snd_system(self, |snd_system| {
      snd_system.play_sfx(key);
    });
  }

  fn play_callout(&self, key: &'static str) {
    with_snd_system(self, |snd_system| {
      snd_system.play_callout(key);
    });
  }
}

fn with_snd_system<T>(ctx: &Context, f: impl FnOnce(&mut SoundSystem) -> T) {
  if let Some(mut system) = ctx.systems.get::<SoundSystem>() {
    f(&mut system);
  } else {
    log::error!("SoundSystem not running.");
  }
}
