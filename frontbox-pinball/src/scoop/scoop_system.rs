use std::any::type_name;
use std::collections::HashSet;
use std::marker::PhantomData;

use frontbox_core::prelude::*;
use frontbox_scene::{Completes, PlayableScene, ScenePlayer, SceneSystemExt};

use crate::scoop::{ScoopBallEntered, ScoopEjectStarted, ScoopEjected, ScoopEntry};
use crate::{BallManagementExt, BallSearching, PlayerTilted, TurnEnding};

/// A scoop (saucer, kickout hole): a hole with a switch that a ball comes to rest on, and a coil that kicks it back
/// out. Ejects any ball that lands in it, at any time (in a turn, after a tilt, at boot), and kicks again if the ball
/// is still there afterwards.
///
/// ## One per scoop
/// `S` is a marker type that names the scoop, so a machine has a `ScoopSystem` per scoop, each with its own events:
///
/// ```rust,ignore
/// pub struct Lower;
/// pub struct Rear;
///
/// app.system(
///   ScoopSystem::<Lower>::new(lower_scoop::SWITCH.name, lower_scoop::COIL.name)
///     .lower_entrance_switch(subway::SWITCH.name)
///     .before_eject_scene(eject_scene),
/// );
/// app.system(ScoopSystem::<Rear>::new(rear_scoop::SWITCH.name, rear_scoop::COIL.name));
///
/// // in on_event
/// if event.is::<ScoopEjected<Lower>>() { /* ... */ }
/// ```
///
/// ## What happens to a ball
/// Two things decide it, and they're separate:
/// - **Whether the ball can go** is decided for each ball as it lands, by the game: other systems hold it
///   ([`Self::hold`]) for as long as they need it, and it can go once none of them does. The scoop has no setting for
///   this, since the same scoop might hold one ball for a menu and let the next go.
/// - **How the eject goes** is set once, on the scoop: with a [`before_eject_scene`](Self::before_eject_scene), the
///   scene plays through and then the coil fires. Without one, the coil fires right away.
///
/// For every ball:
/// 1. It lands, and `ScoopBallEntered` is emitted.
/// 2. Any system that wants it to wait calls [`Self::hold`] on hearing that.
/// 3. Once nothing holds it (right away, if nothing did), `ScoopEjectStarted` is emitted and the eject scene plays.
///    A hold placed while it plays stops the eject, which starts over from here when the ball is next let go.
/// 4. The coil fires, and `ScoopEjected` is emitted.
///
/// So a scoop that only ever ejects needs nothing more than its switch and coil, and a scoop that opens a menu is the
/// same scoop plus a menu system that holds the ball while it's open.
///
/// ## Holds
/// A hold is kept by the system that placed it, by its `ctx.id()`, so several can hold the same ball (an award's
/// animation, then a menu, then a mode's intro), and the ball goes when the last of them calls [`Self::release`]. One
/// system can hand the ball to another, as a release only takes effect after the systems hearing about it have had
/// the chance to hold.
///
/// A tilt or the end of a turn clears every hold, and a system's hold is cleared if that system is despawned.
///
/// ```rust,ignore
/// // on ScoopBallEntered<Lower>
/// ctx.systems.expect::<ScoopSystem<Lower>>().hold(ctx.id(), ctx.into());
///
/// // once the menu closes
/// ctx.systems.expect::<ScoopSystem<Lower>>().release(ctx.id(), ctx.into());
/// ```
///
/// ## Ball search
/// List it in the `BallSearching` routine to have it kick during a search, unless its ball is being held or is about
/// to eject.
///
/// ## Ball counts
/// A ball resting in the scoop is reported to ball management as held (`set_balls_held`). It's still in play, so a
/// drain while it's here doesn't end the turn, but a ball search doesn't look for it.
///
/// ## Its own coil
/// For [`KICK_SETTLE`] after the coil fires, the switch closing isn't taken for a ball landing: the kicker can pass
/// through an opto when it fires into an empty scoop (as a ball search does), and each phantom ball would be kicked
/// again. If the switch is still closed once that time is up, a ball did land. A ball still there after a kick is kicked again after the `verify_delay`, waiting twice as long after each
/// kick that fails, up to [`MAX_VERIFY_DELAY`], so a ball stuck for good doesn't fire the coil every second.
pub struct ScoopSystem<S> {
  handle: SystemHandle,
  /// For logs
  name: &'static str,
  occupancy_switch: &'static str,
  coil: &'static str,
  lower_entrance_switch: Option<&'static str>,
  lower_entrance_timeout: Duration,
  verify_delay: Duration,
  before_eject_scene: Option<ScenePlayer<Completes>>,
  after_eject_scene: Option<PlayableScene<Completes>>,
  /// Who's holding the ball, by `ctx.id()`
  holds: HashSet<u64>,
  /// There's a ball that hasn't been ejected yet
  ball_waiting: bool,
  /// Running while the eject scene plays
  eject_cue: Option<u64>,
  /// A ball passed the lower entrance switch and hasn't arrived yet
  lower_entrance_passed: bool,
  lower_entrance_cue: Option<u64>,
  /// Running for [`KICK_SETTLE`] after the coil fires, while the switch closing is the kicker rather than a ball
  settle_cue: Option<u64>,
  /// The switch closed while settling, so a ball may have landed under the kicker
  closed_while_settling: bool,
  /// Kicks in a row that left the ball in the scoop, to back off the next one
  failed_kicks: u32,
  scoop: PhantomData<fn() -> S>,
}

/// How long after the coil fires its switch closing isn't taken for a ball landing
pub const KICK_SETTLE: Duration = Duration::from_millis(250);

/// The longest wait between kicks at a ball that won't leave
pub const MAX_VERIFY_DELAY: Duration = Duration::from_secs(30);

impl<S: 'static> ScoopSystem<S> {
  /// `occupancy_switch` is closed while a ball rests in the scoop, and `coil` kicks it out
  pub fn new(occupancy_switch: &'static str, coil: &'static str) -> Self {
    let name = type_name::<S>();
    Self {
      handle: SystemHandle::default(),
      name: name.rsplit("::").next().unwrap_or(name),
      occupancy_switch,
      coil,
      lower_entrance_switch: None,
      lower_entrance_timeout: Duration::from_secs(5),
      verify_delay: Duration::from_secs(1),
      before_eject_scene: None,
      after_eject_scene: None,
      holds: HashSet::new(),
      ball_waiting: false,
      eject_cue: None,
      lower_entrance_passed: false,
      lower_entrance_cue: None,
      settle_cue: None,
      closed_while_settling: false,
      failed_kicks: 0,
      scoop: PhantomData,
    }
  }

  /// A switch a ball passes on its way into the scoop from below the playfield (e.g. through a subway), for scoops
  /// that have such an entrance. A ball arriving after it is reported as
  /// `ScoopEntry::LowerEntrance`.
  pub fn lower_entrance_switch(mut self, switch: &'static str) -> Self {
    self.lower_entrance_switch = Some(switch);
    self
  }

  /// How long a ball has to get from the lower entrance switch to the occupancy switch and still count as entering
  /// that way (default 5s)
  pub fn lower_entrance_timeout(mut self, timeout: Duration) -> Self {
    self.lower_entrance_timeout = timeout;
    self
  }

  /// How long after a kick the scoop is checked, and kicked again if the ball is still there (default 1s)
  pub fn verify_delay(mut self, delay: Duration) -> Self {
    self.verify_delay = delay;
    self
  }

  /// A scene to play before the coil fires, which it does once the scene has played through. `ScoopEjectStarted`
  /// is emitted as it starts. Without one, the coil fires right away.
  ///
  /// It has to be a scene that completes, as the ball would otherwise never eject.
  pub fn before_eject_scene(mut self, scene: PlayableScene<Completes>) -> Self {
    self.before_eject_scene = Some(ScenePlayer::new(scene));
    self
  }

  /// A scene to play each time the coil fires. It plays to its end, whatever the scoop does next.
  pub fn after_eject_scene(mut self, scene: PlayableScene<Completes>) -> Self {
    self.after_eject_scene = Some(scene);
    self
  }

  /// Keep a ball in the scoop, keyed by the requester's `ctx.id()`. This also catches one that's about to eject.
  pub fn hold(&mut self, requester: u64, ctx: &ServiceContext) {
    self.holds.insert(requester);
    if let Some(cue) = self.eject_cue.take() {
      let ctx = &ctx.for_system(self.handle);
      ctx.cues.cancel(cue);
      if let Some(scene) = self.before_eject_scene.as_mut() {
        scene.stop(ctx);
      }
    }
  }

  /// Take back a [`Self::hold`]. The ball ejects once nothing is holding it.
  pub fn release(&mut self, requester: u64, ctx: &ServiceContext) {
    self.holds.remove(&requester);
    // left to the next event, so that a hold being taken over from this one can get in first
    ctx
      .for_system(self.handle)
      .emit(ScoopCue::<S>::new(Step::BeginEject));
  }

  fn is_occupied(&self, ctx: &SystemContext) -> bool {
    ctx
      .switches
      .is_closed(self.occupancy_switch)
      .unwrap_or(false)
  }

  fn on_ball_enter(&mut self, ctx: &SystemContext) {
    let entry = if std::mem::take(&mut self.lower_entrance_passed) {
      ScoopEntry::LowerEntrance
    } else {
      ScoopEntry::Direct
    };
    ctx.cues.cancel(self.lower_entrance_cue.take());
    log::info!(target: "frontbox::scoop", "Ball entered {} scoop ({:?})", self.name, entry);

    self.ball_waiting = true;
    ctx.set_balls_held(1);

    // whatever wants to hold the ball hears about it first, and the eject starts if nothing did
    ctx.emit(ScoopBallEntered::<S>::new(entry));
    ctx.emit(ScoopCue::<S>::new(Step::BeginEject));
  }

  fn on_lower_entrance_switch_closed(&mut self, ctx: &SystemContext) {
    self.lower_entrance_passed = true;
    ctx.cues.cancel(self.lower_entrance_cue.take());
    self.lower_entrance_cue = Some(ctx.cues.start(
      ScoopCue::<S>::new(Step::LowerEntranceExpired),
      self.lower_entrance_timeout.once(),
    ));
  }

  fn begin_eject(&mut self, ctx: &SystemContext) {
    if !self.ball_waiting || self.eject_cue.is_some() || !self.holds.is_empty() {
      return;
    }

    ctx.emit(ScoopEjectStarted::<S>::new());
    let Some(scene) = self.before_eject_scene.as_mut() else {
      self.kick(ctx);
      return;
    };

    scene.restart(ctx);
    let length = scene.scene().length();
    self.eject_cue = Some(ctx.cues.start(
      ScoopCue::<S>::new(Step::EjectSceneCompleted),
      length.once(),
    ));
  }

  /// Fire the coil, then check the scoop is empty once the kick has had time to work
  fn fire(&mut self, ctx: &SystemContext) {
    ctx.drivers.activate(self.coil, ActivationMode::Tap);
    ctx.cues.cancel(self.settle_cue.take());
    self.settle_cue = Some(ctx.cues.start(ScoopCue::<S>::new(Step::KickSettled), KICK_SETTLE.once()));

    // so the check reads the switch as it is, even if its opening went unreported
    if let Some(machine) = ctx.systems.get::<MachineSystem>() {
      machine.refresh_switch_state();
    }
    let delay = self
      .verify_delay
      .saturating_mul(1 << self.failed_kicks.min(16))
      .min(MAX_VERIFY_DELAY.max(self.verify_delay));
    ctx.cues.start(ScoopCue::<S>::new(Step::VacancyCheckDue), delay.once());
  }

  fn kick(&mut self, ctx: &SystemContext) {
    self.fire(ctx);
    self.eject_cue = None;
    if let Some(scene) = self.before_eject_scene.as_mut() {
      scene.stop(ctx);
    }
    if let Some(scene) = &self.after_eject_scene {
      ctx.play_scene(scene);
    }
    if self.ball_waiting {
      self.ball_waiting = false;
      ctx.emit(ScoopEjected::<S>::new());
    }
  }

  fn on_vacancy_check_due(&mut self, ctx: &SystemContext) {
    // a ball that's waiting came in since the kick, and ejects in its own time
    if !self.ball_waiting && self.holds.is_empty() && self.is_occupied(ctx) {
      self.failed_kicks += 1;
      log::warn!(
        target: "frontbox::scoop",
        "Ball still in {} scoop after {} kick(s). Kicking again.",
        self.name,
        self.failed_kicks
      );
      self.kick(ctx);
    }
  }

  fn on_switch_closed(&mut self, ctx: &SystemContext) {
    if self.settle_cue.is_some() {
      log::debug!(target: "frontbox::scoop", "{} scoop switch closed as the coil fired, which may not be a ball", self.name);
      self.closed_while_settling = true;
      return;
    }
    self.on_ball_enter(ctx);
  }

  /// The kicker is back, so a switch still closed from while it was moving is a ball that landed
  fn on_kick_settled(&mut self, ctx: &SystemContext) {
    self.settle_cue = None;
    if std::mem::take(&mut self.closed_while_settling) && !self.ball_waiting && self.is_occupied(ctx) {
      self.on_ball_enter(ctx);
    }
  }

  fn clear_holds(&mut self, ctx: &SystemContext) {
    self.holds.clear();
    self.begin_eject(ctx);
  }

  fn on_cue(&mut self, step: Step, ctx: &SystemContext) {
    match step {
      Step::BeginEject => self.begin_eject(ctx),
      Step::EjectSceneCompleted => self.kick(ctx),
      Step::VacancyCheckDue => self.on_vacancy_check_due(ctx),
      Step::KickSettled => self.on_kick_settled(ctx),
      Step::LowerEntranceExpired => {
        self.lower_entrance_passed = false;
        self.lower_entrance_cue = None;
      }
    }
  }
}

impl<S: 'static> System for ScoopSystem<S> {
  fn on_spawn(&mut self, ctx: &SystemContext) {
    self.handle = ctx.handle();

    if self.is_occupied(ctx) {
      log::debug!(target: "frontbox::scoop", "{} scoop is occupied. Ejecting.", self.name);
      ctx.set_balls_held(1);
      self.kick(ctx);
    }
  }

  fn on_routine(&mut self, routine: &dyn Event, ctx: &SystemContext) {
    if routine.is::<BallSearching>() {
      // not while its ball is being held, or is about to eject
      if !self.ball_waiting || (self.holds.is_empty() && self.eject_cue.is_none()) {
        self.fire(ctx);
      }
      ctx.routines.complete_step::<BallSearching>();
    }
  }

  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if event.switch.name == self.occupancy_switch {
        self.on_switch_closed(ctx);
      } else if Some(event.switch.name) == self.lower_entrance_switch {
        self.on_lower_entrance_switch_closed(ctx);
      }
    } else if let Some(event) = event.downcast_ref::<SwitchOpened>()
      && event.switch.name == self.occupancy_switch
    {
      self.failed_kicks = 0;
      ctx.set_balls_held(0);
    } else if let Some(cue) = event.downcast_ref::<ScoopCue<S>>() {
      self.on_cue(cue.step, ctx);
    } else if event.is::<RoutineStarted<TurnEnding>>() || event.is::<PlayerTilted>() {
      self.clear_holds(ctx);
    } else if let Some(SystemDespawned { id, .. }) = event.downcast_ref::<SystemDespawned>()
      && self.holds.remove(id)
    {
      self.begin_eject(ctx);
    }
  }
}

// -- Cues --

/// What the scoop sends itself. Generic like the public events, so one scoop doesn't act on another's.
#[derive(serde::Serialize, Event)]
struct ScoopCue<S> {
  step: Step,
  #[serde(skip)]
  scoop: PhantomData<fn() -> S>,
}

impl<S> ScoopCue<S> {
  fn new(step: Step) -> Self {
    Self {
      step,
      scoop: PhantomData,
    }
  }
}

#[derive(serde::Serialize, Clone, Copy)]
enum Step {
  /// Start the eject, if there's a ball and nothing is holding it
  BeginEject,
  EjectSceneCompleted,
  VacancyCheckDue,
  /// The kick has had time to work, so the switch closing is a ball again
  KickSettled,
  LowerEntranceExpired,
}
