use std::sync::atomic::{AtomicU64, Ordering};

use frontbox_core::prelude::*;
use serde::Serialize;

use crate::*;

/// Plays scenes: holds a [`Playback`] for each one playing and moves their clocks. Systems that show a kind of layer read
/// the playbacks from here each frame: `LedSystem` draws their LED layers.
///
/// A system plays a scene one of two ways:
///
/// - **Handing it off** with [`play_scene`](SceneSystemExt::play_scene), for a response such as a flash on a hit. No
///   system owns it, so it plays to its end whatever happens to the system that asked.
/// - **Owning it** with a [`ScenePlayer`], for a scene the system turns on and off. It's shown while its owner is
///   active: when the owner deactivates it pauses and is hidden until the owner reactivates, and when the owner despawns
///   it's removed.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_scene::*;
/// # fn example(ctx: &SystemContext, flash: PlayableScene<Completes>) {
/// ctx.play_scene(&flash);
/// # }
/// ```
pub struct SceneSystem {
  /// In the order they started, which is also the order they're drawn in on the same z
  playbacks: Vec<HeldPlayback>,
}

impl SceneSystem {
  pub fn new() -> Self {
    Self { playbacks: Vec::new() }
  }

  /// Plays a scene to its end, owned by no system
  pub fn play(&mut self, scene: &PlayableScene<Completes>) -> PlaybackId {
    let id = PlaybackId::next();
    log::debug!(target: "frontbox::scene", "Playing {id:?} to its end");
    self.playbacks.push(HeldPlayback { id, owner: None, owner_active: true, playback: Playback::new(scene) });
    id
  }

  /// Starts a scene for `owner`, the id of the system it belongs to. [`ScenePlayer`] does this for a system.
  pub fn start<C>(&mut self, owner: u64, scene: &PlayableScene<C>) -> PlaybackId {
    let id = PlaybackId::next();
    log::debug!(target: "frontbox::scene", "System {owner} started {id:?}");
    self.playbacks.push(HeldPlayback { id, owner: Some(owner), owner_active: true, playback: Playback::new(scene) });
    id
  }

  /// Removes a playback. Nothing happens if it's already gone.
  pub fn remove(&mut self, id: PlaybackId) {
    self.playbacks.retain(|held| held.id != id);
  }

  /// A playback that's still here, to change its params
  pub fn playback(&mut self, id: PlaybackId) -> Option<&mut Playback> {
    self.playbacks.iter_mut().find(|held| held.id == id).map(|held| &mut held.playback)
  }

  /// Whether a playback is still held. One goes when it's removed, when it completes unless it holds its last moment,
  /// and with its owner.
  pub fn contains(&self, id: PlaybackId) -> bool {
    self.playbacks.iter().any(|held| held.id == id)
  }

  /// The playbacks to draw this frame, in the order they started
  pub fn shown(&self) -> impl Iterator<Item = (PlaybackId, &Playback)> {
    self.playbacks.iter().filter(|held| held.owner_active && held.playback.is_shown()).map(|held| (held.id, &held.playback))
  }

  fn owned_by(&mut self, owner: u64) -> impl Iterator<Item = &mut HeldPlayback> {
    self.playbacks.iter_mut().filter(move |held| held.owner == Some(owner))
  }
}

impl System for SceneSystem {
  fn on_tick(&mut self, delta: Duration, ctx: &SystemContext) {
    for held in self.playbacks.iter_mut().filter(|held| held.owner_active) {
      let was_complete = held.playback.is_complete();
      held.playback.advance(delta);
      if !was_complete && held.playback.is_complete() {
        ctx.emit(ScenePlaybackCompleted { playback: held.id });
      }
    }
    self.playbacks.retain(|held| held.playback.is_shown());
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if let Some(SystemDespawned { id, .. }) = event.downcast_ref::<SystemDespawned>() {
      self.playbacks.retain(|held| held.owner != Some(*id));
    } else if let Some(SystemDeactivated { id, .. }) = event.downcast_ref::<SystemDeactivated>() {
      self.owned_by(*id).for_each(|held| held.owner_active = false);
    } else if let Some(SystemReactivated { id, .. }) = event.downcast_ref::<SystemReactivated>() {
      self.owned_by(*id).for_each(|held| held.owner_active = true);
    }
  }
}

/// A playback held by the [`SceneSystem`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct PlaybackId(u64);

impl PlaybackId {
  fn next() -> Self {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    Self(NEXT.fetch_add(1, Ordering::Relaxed))
  }

  /// An id no playback has, for one that couldn't be started
  pub(crate) fn none() -> Self {
    Self(0)
  }
}

struct HeldPlayback {
  id: PlaybackId,
  /// `None` for one that was handed off to play to its end
  owner: Option<u64>,
  owner_active: bool,
  playback: Playback,
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::animation::Cycle;
  use frontbox_gfx::scene::{EndBehavior, Timing};

  use super::*;

  const OWNER: u64 = 7;

  fn timed(duration: f32, cycle: Cycle) -> PlayableScene {
    PlayableScene::new(Scene::new(vec![]).timing(Timing::new(duration, cycle)))
  }

  fn completing(duration: f32) -> PlayableScene<Completes> {
    PlayableScene::completing(Scene::new(vec![]).timing(Timing::new(duration, Cycle::Once))).unwrap()
  }

  fn tick(context: &TestContext, delta: Duration) {
    context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| scenes.on_tick(delta, &context.sys_ctx()));
  }

  fn send(context: &TestContext, event: impl Event) {
    context.sys_ctx().systems.with::<SceneSystem, _>(|scenes| scenes.on_event(&event, &context.sys_ctx()));
  }

  fn with_scenes<R>(context: &TestContext, f: impl FnOnce(&mut SceneSystem) -> R) -> R {
    context.sys_ctx().systems.with::<SceneSystem, _>(f).unwrap()
  }

  fn start(context: &TestContext, owner: u64, scene: PlayableScene) -> PlaybackId {
    with_scenes(context, |scenes| scenes.start(owner, &scene))
  }

  fn shown(context: &TestContext) -> Vec<PlaybackId> {
    with_scenes(context, |scenes| scenes.shown().map(|(id, _)| id).collect())
  }

  fn context() -> TestContext {
    let mut context = TestContext::default();
    context.insert_system(SceneSystem::new());
    context
  }

  #[test]
  fn playbacks_are_shown_in_the_order_they_started() {
    let context = context();
    let first = start(&context, OWNER, timed(1.0, Cycle::Forever));
    let second = start(&context, OWNER, timed(1.0, Cycle::Forever));
    assert_eq!(shown(&context), [first, second]);
  }

  #[test]
  fn a_completed_playback_is_removed_and_says_so() {
    let mut context = context();
    let id = start(&context, OWNER, timed(0.5, Cycle::Once));
    tick(&context, Duration::from_secs(1));
    assert!(!with_scenes(&context, |scenes| scenes.contains(id)));
    let events = context.events_emitted();
    let completed: Vec<_> = events.iter().filter_map(|event| event.event.downcast_ref::<ScenePlaybackCompleted>()).collect();
    assert_eq!(completed.iter().map(|event| event.playback).collect::<Vec<_>>(), [id]);
  }

  #[test]
  fn one_that_holds_stays_until_removed() {
    let context = context();
    let id = start(&context, OWNER, Scene::new(vec![]).timing(Timing::new(0.5, Cycle::Once).end(EndBehavior::Hold)).into());
    tick(&context, Duration::from_secs(1));
    assert_eq!(shown(&context), [id]);
    with_scenes(&context, |scenes| scenes.remove(id));
    assert!(shown(&context).is_empty());
  }

  #[test]
  fn an_inactive_owners_playbacks_pause_and_hide() {
    let context = context();
    let id = start(&context, OWNER, timed(1.0, Cycle::Once));
    send(&context, SystemDeactivated::new(OWNER, ROOT_GROUP));
    tick(&context, Duration::from_millis(500));
    assert!(shown(&context).is_empty());

    send(&context, SystemReactivated::new(OWNER, ROOT_GROUP));
    assert_eq!(shown(&context), [id]);
    assert_eq!(with_scenes(&context, |scenes| scenes.playback(id).unwrap().moment().time), 0.0);
  }

  #[test]
  fn a_despawned_owners_playbacks_go_with_it() {
    let context = context();
    let id = start(&context, OWNER, timed(1.0, Cycle::Forever));
    send(&context, SystemDespawned::new(OWNER, ROOT_GROUP));
    assert!(!with_scenes(&context, |scenes| scenes.contains(id)));
  }

  #[test]
  fn a_scene_handed_off_plays_to_its_end_whatever_systems_do() {
    let context = context();
    let id = with_scenes(&context, |scenes| scenes.play(&completing(1.0)));
    send(&context, SystemDeactivated::new(OWNER, ROOT_GROUP));
    tick(&context, Duration::from_millis(500));
    send(&context, SystemDespawned::new(OWNER, ROOT_GROUP));
    assert_eq!(shown(&context), [id]);
    assert_eq!(with_scenes(&context, |scenes| scenes.playback(id).unwrap().moment().time), 0.5);

    tick(&context, Duration::from_millis(600));
    assert!(!with_scenes(&context, |scenes| scenes.contains(id)));
  }

  #[test]
  fn other_owners_are_left_alone() {
    let context = context();
    let id = start(&context, OWNER + 1, timed(1.0, Cycle::Forever));
    send(&context, SystemDeactivated::new(OWNER, ROOT_GROUP));
    send(&context, SystemDespawned::new(OWNER, ROOT_GROUP));
    assert_eq!(shown(&context), [id]);
  }
}
