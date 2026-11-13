use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use fast_protocol::SwitchState;
use tokio::sync::mpsc;

use crate::app::run_loop::harness::Harness;
use crate::machine::vm::{self, VirtualHardware};
use crate::machine::{MachineMessage, MachineSystem};
use crate::prelude::*;
use crate::provided::WatchdogSystem;
use crate::systems::routine::Routines;

/// A whole app running against virtual hardware, driven step by step by a test, with no tokio, real time or boards.
///
/// The app is built as it is for the machine, from the same [`App`], and boots as on the virtual platform: the switches
/// are read from the [`VirtualHardware`], drivers are configured from their modes, and the framework's own systems
/// start first. The test then plays the part of the ball and the player, closing and opening switches and letting time
/// pass, and checks what happened: in the hardware, in the systems, or in the events they emitted.
///
/// Each step runs until everything it set off is done, including any back and forth with the hardware, so a test
/// always sees the settled result.
///
/// ```ignore
/// let mut s = Scenario::builder(app).closed(["trough_1", "trough_2"]).start();
/// let saved = s.record(|e: &BallSaved| e.count);
///
/// s.close("plunge_lane");
/// s.advance(Duration::from_secs(2));
/// assert_eq!(s.hardware().driver("trough_eject").unwrap().taps, 1);
/// assert_eq!(saved.take(), [1]);
/// ```
pub struct Scenario {
  harness: Harness,
  hardware: VirtualHardware,
  machine_rx: mpsc::UnboundedReceiver<MachineMessage>,
  /// The scenario's own system, which `act` acts as
  tester: SystemHandle,
  taps: Taps,
}

impl Scenario {
  /// Set up a scenario for the app. Its platform and the paths it would save to are ignored: the scenario always runs
  /// on virtual hardware and saves nothing. Operator config is loaded when the app is created, so build it with a
  /// `config_path` of `None` unless the test means to read one.
  pub fn new(app: App) -> ScenarioBuilder {
    ScenarioBuilder {
      app,
      closed: Vec::new(),
    }
  }

  /// Close the switch, as a ball or a player would. Panics if there's no such switch or it's already closed.
  pub fn close_switch(&mut self, switch: &str) {
    self.set_switch(switch, SwitchState::Closed);
  }

  /// Open the switch. Panics if there's no such switch or it's already open.
  pub fn open_switch(&mut self, switch: &str) {
    self.set_switch(switch, SwitchState::Open);
  }

  /// Close then open the switch, as a ball rolling over it or a button press would
  pub fn hit_switch(&mut self, switch: &str) {
    self.close_switch(switch);
    self.open_switch(switch);
  }

  /// Lose the connection to the machine, as pulling its USB cable would. The app is told, and from then on nothing it
  /// sends reaches the hardware and switch changes aren't reported. Panics if it's already disconnected.
  pub fn disconnect_machine(&mut self) {
    let disconnected = self.hardware.disconnect().unwrap_or_else(|| panic!("The machine is already disconnected"));
    self.harness.send(disconnected);
    self.settle();
  }

  /// Bring the connection to the machine back after `disconnect_machine`. The hardware comes back as if it lost power,
  /// and the app is told along with the switches' states. Panics if it's still connected.
  pub fn reconnect_machine(&mut self) {
    let reconnected = self.hardware.reconnect().unwrap_or_else(|| panic!("The machine is still connected"));
    self.harness.send(reconnected);
    self.settle();
  }

  /// Let time pass, ticking at the app's interval as the machine would, so cues and timers run as they would there
  pub fn advance(&mut self, duration: Duration) {
    let interval = self.harness.tick_interval();
    assert!(!interval.is_zero(), "The app's system interval is zero, so time can't pass");
    let mut left = duration;
    while !left.is_zero() {
      let delta = left.min(interval);
      self.harness.tick_by(delta);
      self.settle();
      left -= delta;
    }
  }

  /// Broadcast the event, as if a system had
  pub fn emit(&mut self, event: impl Event) {
    self.harness.emit(event);
    self.settle();
  }

  /// Call into the app's systems or read its state, e.g. to start a game or check one is running. `f` runs as a system
  /// of the scenario's own, so what it emits comes from that system and it can look up any of the app's. What `f` sets
  /// off runs before this returns.
  pub fn act<R>(&mut self, f: impl FnOnce(&SystemContext) -> R) -> R {
    let result = f(&self.harness.service_context().for_system(self.tester));
    self.settle();
    result
  }

  /// Record each `E` emitted from now on, as `record` maps it, e.g. `s.record(|e: &BallSaved| e.count)`. Use
  /// `|_: &E| ()` to just count them.
  pub fn record<E: Event, T: 'static>(&mut self, record: impl Fn(&E) -> T + 'static) -> Recording<T> {
    let recording = Recording::default();
    let into = recording.clone();
    self.taps.borrow_mut().push(Box::new(move |event| {
      if let Some(event) = event.downcast_ref::<E>() {
        into.0.borrow_mut().push(record(event));
      }
    }));
    recording
  }

  /// The hardware as the app has left it: switches, how each driver is configured and triggered, and the watchdog
  pub fn hardware(&self) -> &VirtualHardware {
    &self.hardware
  }

  fn set_switch(&mut self, name: &str, state: SwitchState) {
    let id = self
      .hardware
      .switch_id(name)
      .unwrap_or_else(|| panic!("There's no switch named {name}"));
    if self.hardware.is_closed(name) == Some(matches!(state, SwitchState::Closed)) {
      panic!("Switch {name} is already {state:?}");
    }
    // a disconnected machine doesn't report it
    if let Some(change) = self.hardware.set_switch(id, state) {
      self.harness.send(change);
    }
    self.settle();
  }

  /// Run the app and the hardware until neither has anything left to do
  fn settle(&mut self) {
    loop {
      self.harness.settle();
      let mut replied = false;
      while let Ok(message) = self.machine_rx.try_recv() {
        for reply in self.hardware.apply(&message) {
          self.harness.send(reply);
          replied = true;
        }
      }
      if !replied {
        return;
      }
    }
  }
}

/// Sets up a [`Scenario`], from [`Scenario::builder`]
pub struct ScenarioBuilder {
  app: App,
  closed: Vec<&'static str>,
}

impl ScenarioBuilder {
  /// Switches that are closed when the machine boots, such as the trough's for the balls in it
  pub fn closed(mut self, switches: impl IntoIterator<Item = &'static str>) -> Self {
    self.closed.extend(switches);
    self
  }

  /// Boot the app, as `App::run` does on the virtual platform, and settle what its systems do on spawn
  pub fn start(self) -> Scenario {
    let parts = self.app.into_parts();
    let app_config = AppConfig {
      planes: parts.planes,
      ..AppConfig::from_boot_config(&parts.boot_config)
    };

    let mut virtual_hardware = VirtualHardware::new(&parts.boot_config.io_network);
    for name in self.closed {
      let id = virtual_hardware
        .switch_id(name)
        .unwrap_or_else(|| panic!("There's no switch named {name}"));
      virtual_hardware.set_switch(id, SwitchState::Closed);
    }
    let mut hardware = vm::read_hardware(parts.boot_config, &virtual_hardware);
    app_config.planes.iter().for_each(|plane| hardware.leds.add_plane(plane.plane));
    let state = AppState::from_hardware(hardware, parts.configs, app_config);
    virtual_hardware.configure_drivers(&state);

    let (machine_tx, machine_rx) = mpsc::unbounded_channel();
    let (app_tx, app_rx) = mpsc::unbounded_channel();
    let bridge = SystemContainer::new(MachineSystem::new(machine_tx, app_tx.clone()));
    let harness = Harness::with(state, Routines::new(parts.routines), bridge.id(), (app_tx, app_rx));

    let taps = Taps::default();
    let mut scenario = Scenario {
      harness,
      hardware: virtual_hardware,
      machine_rx,
      tester: SystemHandle::default(),
      taps: taps.clone(),
    };

    // in the order `App::run` spawns them, after the tester so it hears everything from the start
    scenario.tester = scenario.harness.spawn(Tester(taps));
    scenario.harness.spawn(bridge);
    for system in parts.systems {
      scenario.harness.spawn(system);
    }
    scenario.harness.spawn(WatchdogSystem::new());
    scenario.settle();
    scenario
  }
}

/// What a [`Scenario`] recorded, from [`Scenario::record`]
pub struct Recording<T>(Rc<RefCell<Vec<T>>>);

impl<T> Recording<T> {
  /// Everything recorded since the last `take`, in the order it was emitted
  pub fn take(&self) -> Vec<T> {
    std::mem::take(&mut *self.0.borrow_mut())
  }
}

impl<T> Clone for Recording<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> Default for Recording<T> {
  fn default() -> Self {
    Self(Rc::default())
  }
}

type Taps = Rc<RefCell<Vec<Box<dyn Fn(&dyn Event)>>>>;

/// The scenario's own system in the app: it hands every event to the scenario's recordings, and is who `act` acts as
struct Tester(Taps);

impl System for Tester {
  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    for tap in self.0.borrow().iter() {
      tap(event);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use super::*;
  use crate::machine::DriverTrigger;

  static START: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new("start").build());
  static TROUGH: LazyLock<SwitchDefinition> = LazyLock::new(|| SwitchDefinition::new("trough").inverted().build());
  static KICKER: LazyLock<DriverDefinition> =
    LazyLock::new(|| DriverDefinition::new("kicker").mode(DriverMode::pulse().build()).build());

  fn app() -> App {
    let mut app = App::new(BootConfig {
      io_network: IoNetwork::new(vec![
        IoBoard::custom(8, 8)
          .wire_switch(0, &START)
          .wire_switch(1, &TROUGH)
          .wire_driver(0, &KICKER),
      ]),
      config_path: None,
      metrics_path: None,
      content_path: None,
      system_interval: Duration::from_millis(10),
      ..BootConfig::default()
    });
    app.system(Kicker::default());
    app
  }

  /// Kicks when start closes, and a second after that says it's done
  #[derive(Default)]
  struct Kicker {
    kicks: u32,
  }

  #[derive(serde::Serialize, Event)]
  struct Kicked(u32);

  #[derive(serde::Serialize, Event)]
  struct KickSettled;

  /// The cue `Kicker` sends itself
  #[derive(serde::Serialize, Event)]
  struct SettleTimer;

  impl Kicker {
    fn kicks(&self) -> u32 {
      self.kicks
    }
  }

  impl System for Kicker {
    fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
      if let Some(SwitchClosed { switch }) = event.downcast_ref::<SwitchClosed>()
        && switch.name == "start"
      {
        self.kicks += 1;
        ctx.drivers.activate("kicker", ActivationMode::Tap);
        ctx.emit(Kicked(self.kicks));
        ctx.cues.start(SettleTimer, Cue::Once(Duration::from_secs(1)));
      } else if event.is::<SettleTimer>() {
        ctx.emit(KickSettled);
      }
    }
  }

  #[test]
  fn it_boots_with_drivers_configured_and_the_watchdog_running() {
    let s = Scenario::new(app()).start();
    let kicker = s.hardware().driver("kicker").unwrap();
    assert!(matches!(kicker.config, Some(fast_protocol::DriverConfig::Pulse { .. })));
    assert_eq!(kicker.trigger, DriverTrigger::Off);
    assert!(s.hardware().watchdog_enabled());
  }

  #[test]
  fn switches_closed_at_boot_are_read_by_the_app() {
    let mut s = Scenario::new(app()).closed(["trough"]).start();
    assert_eq!(s.act(|ctx| ctx.switches.is_closed("trough")), Some(true));
    assert_eq!(s.act(|ctx| ctx.switches.is_closed("start")), Some(false));
  }

  #[test]
  fn a_switch_closing_reaches_the_systems_and_their_drivers_reach_the_hardware() {
    let mut s = Scenario::new(app()).start();
    let kicked = s.record(|e: &Kicked| e.0);

    s.hit_switch("start");
    assert_eq!(kicked.take(), [1]);
    assert_eq!(s.hardware().driver("kicker").unwrap().taps, 1);
    assert_eq!(s.act(|ctx| ctx.switches.is_closed("start")), Some(false));
  }

  #[test]
  fn time_passing_runs_cues() {
    let mut s = Scenario::new(app()).start();
    let settled = s.record(|_: &KickSettled| ());

    s.close_switch("start");
    s.advance(Duration::from_millis(990));
    assert!(settled.take().is_empty());
    s.advance(Duration::from_millis(20));
    assert_eq!(settled.take().len(), 1);
  }

  #[test]
  fn act_reaches_the_apps_systems() {
    let mut s = Scenario::new(app()).start();
    s.hit_switch("start");
    s.hit_switch("start");
    assert_eq!(s.act(|ctx| ctx.systems.with::<Kicker, _>(|k| k.kicks())), Some(2));
  }

  #[test]
  fn emitted_events_are_recorded_in_order() {
    let mut s = Scenario::new(app()).start();
    let kicked = s.record(|e: &Kicked| e.0);
    s.emit(Kicked(7));
    s.hit_switch("start");
    assert_eq!(kicked.take(), [7, 1]);
    assert!(kicked.take().is_empty());
  }

  #[test]
  fn a_disconnect_reaches_the_systems() {
    let mut s = Scenario::new(app()).start();
    let disconnected = s.record(|_: &MachineDisconnected| ());
    assert!(s.act(|ctx| ctx.systems.expect::<MachineSystem>().is_connected()));

    s.disconnect_machine();
    assert_eq!(disconnected.take().len(), 1);
    assert!(!s.act(|ctx| ctx.systems.expect::<MachineSystem>().is_connected()));
    assert!(!s.hardware().watchdog_enabled());
  }

  #[test]
  fn once_disconnected_switches_and_drivers_dont_get_through() {
    let mut s = Scenario::new(app()).start();
    let kicked = s.record(|e: &Kicked| e.0);
    s.disconnect_machine();

    s.close_switch("start");
    assert!(kicked.take().is_empty());
    assert_eq!(s.act(|ctx| ctx.switches.is_closed("start")), Some(false));

    s.act(|ctx| ctx.drivers.activate("kicker", ActivationMode::Tap));
    assert_eq!(s.hardware().driver("kicker").unwrap().taps, 0);
  }

  #[test]
  fn a_reconnect_reaches_the_systems() {
    let mut s = Scenario::new(app()).start();
    let reconnected = s.record(|_: &MachineReconnected| ());
    s.disconnect_machine();

    s.reconnect_machine();
    assert_eq!(reconnected.take().len(), 1);
    assert!(s.act(|ctx| ctx.systems.expect::<MachineSystem>().is_connected()));
    assert!(s.hardware().watchdog_enabled());
  }

  #[test]
  fn a_reconnect_configures_and_triggers_drivers_as_they_were() {
    let mut s = Scenario::new(app()).start();
    s.act(|ctx| ctx.drivers.activate("kicker", ActivationMode::VirtualSwitchOn));
    s.disconnect_machine();

    s.reconnect_machine();
    let kicker = s.hardware().driver("kicker").unwrap();
    assert!(matches!(kicker.config, Some(fast_protocol::DriverConfig::Pulse { .. })));
    assert_eq!(kicker.trigger, DriverTrigger::On);

    s.act(|ctx| ctx.drivers.activate("kicker", ActivationMode::Tap));
    assert_eq!(s.hardware().driver("kicker").unwrap().taps, 1);
  }

  #[test]
  fn switches_that_changed_while_disconnected_are_read_without_events() {
    let mut s = Scenario::new(app()).start();
    let kicked = s.record(|e: &Kicked| e.0);
    s.disconnect_machine();
    s.close_switch("start");

    s.reconnect_machine();
    assert_eq!(s.act(|ctx| ctx.switches.is_closed("start")), Some(true));
    assert!(kicked.take().is_empty());

    // and from then on it's reported as usual
    s.open_switch("start");
    s.close_switch("start");
    assert_eq!(kicked.take(), [1]);
  }

  #[test]
  #[should_panic(expected = "still connected")]
  fn reconnecting_while_connected_is_a_mistake() {
    let mut s = Scenario::new(app()).start();
    s.reconnect_machine();
  }

  #[test]
  #[should_panic(expected = "already disconnected")]
  fn disconnecting_twice_is_a_mistake() {
    let mut s = Scenario::new(app()).start();
    s.disconnect_machine();
    s.disconnect_machine();
  }

  #[test]
  #[should_panic(expected = "already Closed")]
  fn closing_a_closed_switch_is_a_mistake() {
    let mut s = Scenario::new(app()).closed(["start"]).start();
    s.close_switch("start");
  }

  #[test]
  #[should_panic(expected = "no switch named nope")]
  fn an_unknown_switch_is_a_mistake() {
    let mut s = Scenario::new(app()).start();
    s.hit_switch("nope");
  }
}
