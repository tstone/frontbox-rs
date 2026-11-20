use std::collections::{HashMap, HashSet};

use image::{Pixel, Rgba};
use itertools::Itertools;

use frontbox_core::prelude::*;
use frontbox_gfx::prelude::*;
use frontbox_scene::{PlaybackId, Scene, SceneSystem};

use crate::*;

const LED_SET_BATCH_SIZE: usize = 24;

/// The brightness, from 0 to 1, at or above which a driver light is on, unless [`LayeredLightManager::threshold`]
/// changes it
pub const DEFAULT_THRESHOLD: f32 = 0.5;

/// One on and off of a flashing declaration. At 2Hz it stays well under the tick rate, so it doesn't alias.
pub const FLASH_PERIOD: Duration = Duration::from_millis(500);

/// # LayeredLightManager
///
/// The [`LightManager`] this crate provides. Each light shows what's declared and played on it, composed in z-layers.
///
/// ### Features
/// - **Conflict Resolution** -- Multiple systems can declare a color on the same layer for a light and the LayeredLightManager will handle resolving that conflict automatically (conflict resolution mode is user settable)
/// - **Layer (z-index) Support** -- It's possible to keep an "under layer" active while playing temporary animations a layer above
/// - **Alpha Compositing** -- lights are rendered in RGBA which supports transparency (under colors show through partially)
/// - An easy way to de-activate light declarations when a system is de-activated
/// - Automatic clearing of unset lights per frame
///
/// ### In Use
///
/// Light management works by way of a _declaration_. A declaration doesn't forcibly set a light, instead it's more like a request, "Hello, I am system 12345 and would prefer for this light to be this color at this level of priority" (you can think of Z-index layers as levels of priority). Each render frame, the LayeredLightManager looks through all active declarations, chooses the highest priority one, resolves any conflicting declarations, and updates the state of lights that need to change. This process also detects lights that are no longer set and clears them automatically.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_lighting::prelude::*;
/// # mod lights {
/// #   use frontbox_core::prelude::*;
/// #   hardware_defs! {
/// #     pub EXAMPLE: LedDefinition = LedDefinition::single("example");
/// #     pub EX1: LedDefinition = LedDefinition::single("ex1");
/// #     pub EX2: LedDefinition = LedDefinition::single("ex2");
/// #     pub EX3: LedDefinition = LedDefinition::single("ex3");
/// #   }
/// # }
/// # fn example(ctx: &SystemContext) {
/// // declare lights by name, at z 3...
/// ctx.declare_lights(&lights::EXAMPLE.q(), 3, Rgba::yellow());
///
/// // ...or by group
/// ctx.declare_lights(
///   &vec![&lights::EX1.q(), &lights::EX2.q(), &lights::EX3.q()],
///   0,
///   Rgba::red()
/// );
///
/// // ...or with a color per light
/// ctx.declare_light_colors(
///   &vec![&lights::EX1.q(), &lights::EX2.q(), &lights::EX3.q()],
///   0,
///   |count| (0..count).map(|i| Rgba::red().mix_with(Rgba::yellow(), i as f32 / count as f32)).collect(),
/// );
/// # }
/// ```
///
/// `declare_lights` takes an `LightQ` (or anything else that resolves to a set of lights), which is the reason for `.q()`. More about this later.
///
/// Later on if these declarations need to be temporarily suspended because the System is going inactive, they can be temporarily disabled:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_lighting::prelude::*;
/// # fn example(ctx: &SystemContext) {
/// ctx.deactivate_light_declarations();
/// # }
/// ```
///
/// In fact, this behavior is built-in to `System` by default. When a system goes inactive, if `LayeredLightManager` is live, it will de-activate declarations, then re-activate them once the System comes back.
///
/// ### Layering
///
/// It is possible to declare multiple layers for the same light. If higher layers are opaque they will be rendered. If higher layers are transparent, they will render with a degree of "see-through" to layers below them.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_lighting::prelude::*;
/// # mod lights {
/// #   use frontbox_core::prelude::*;
/// #   hardware_defs! {
/// #     pub EXAMPLE: LedDefinition = LedDefinition::single("example");
/// #   }
/// # }
/// # fn example(ctx: &SystemContext) {
/// // higher layer declares 50% transparent red
/// ctx.declare_lights(&lights::EXAMPLE.q(), 1, Rgba::red().with_alpha_f32(0.5));
///
/// // over top of white
/// ctx.declare_lights(&lights::EXAMPLE.q(), 0, Rgba::white());
///
/// // final color renders as pink [255, 127, 127, 255]
/// # }
/// ```
///
/// ### Driver lights
///
/// A driver light ([`DriverDefinition::light`]) is either on or off. It takes its color like any other light, and is on
/// when that color's brightness, its largest channel after compositing over black, is at or above its threshold.
/// The threshold is [`DEFAULT_THRESHOLD`] unless set with [`threshold`](Self::threshold) or
/// [`threshold_for`](Self::threshold_for).

pub struct LayeredLightManager {
  all_lights: Vec<LightAddress>,
  // Rule: Systems cannot contradict themselves. Declarations are thus unique by light/system/z-index.
  declarations: HashMap<LightAddress, HashMap<DeclarationIdentifier, StatefulLightDeclaration>>,
  /// What each LED address was last set to, so only those that change are sent
  prior_render: HashMap<LedAddress, Rgb>,
  /// Whether each driver light was last turned on, so only those that change are sent
  prior_drivers: HashMap<&'static str, bool>,
  threshold: f32,
  /// Thresholds for some lights, resolved to `thresholds` on spawn. A light more than one reaches takes the last.
  threshold_queries: Vec<(LightQ, f32)>,
  thresholds: HashMap<LightAddress, f32>,
  conflict_resolution: HashMap<LightAddress, LightConflictResolution>,
  alternate_resolver: AlternateResolver,
  /// The `order` the next new declaration gets
  next_order: u64,
  /// Where each shown playback's light layers land, worked out the first time it's shown, as lights don't move
  placements: HashMap<PlaybackId, LightPlacement>,
  /// This frame's colors from playbacks, by light: a z and a color for each that reaches it, in the order they started
  played: HashMap<LightAddress, Vec<(i8, Rgba<u8>)>>,
  /// How far into [`FLASH_PERIOD`] flashing declarations are
  flash_clock: Duration,
}

impl LayeredLightManager {
  pub fn new() -> Self {
    Self {
      declarations: HashMap::new(),
      all_lights: Vec::new(),
      prior_render: HashMap::new(),
      prior_drivers: HashMap::new(),
      threshold: DEFAULT_THRESHOLD,
      threshold_queries: Vec::new(),
      thresholds: HashMap::new(),
      conflict_resolution: HashMap::new(),
      alternate_resolver: AlternateResolver::new(),
      next_order: 0,
      placements: HashMap::new(),
      played: HashMap::new(),
      flash_clock: Duration::ZERO,
    }
  }

  /// The brightness, from 0 to 1, at or above which driver lights are on. [`DEFAULT_THRESHOLD`] if not set.
  pub fn threshold(mut self, threshold: f32) -> Self {
    self.threshold = threshold;
    self
  }

  /// The threshold for the driver lights `lights` reaches, in place of [`threshold`](Self::threshold)
  pub fn threshold_for(mut self, lights: impl Into<LightQ>, threshold: f32) -> Self {
    self.threshold_queries.push((lights.into(), threshold));
    self
  }

  /// Whether flashing declarations are in the on half of their period
  fn flash_on(&self) -> bool {
    self.flash_clock < FLASH_PERIOD / 2
  }

  #[allow(unused)]
  pub(crate) fn declarations_for(&self, light: &LightAddress) -> Vec<StatefulLightDeclaration> {
    self
      .declarations
      .get(light)
      .map(|ds| ds.values().copied().collect())
      .unwrap_or(Vec::new())
  }

  pub fn reset(&mut self) {
    self.declarations.clear();
    self.prior_render.clear();
    self.prior_drivers.clear();
    self.conflict_resolution.clear();
    self.alternate_resolver.reset();
  }

  fn declare_inner(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], mode: LightMode, active: bool) {
    log::trace!(target: "frontbox::lighting", "System {owner} declared at z {z} ({mode:?}): {colors:?}");

    let id = DeclarationIdentifier { system_id: owner, z_index: z };
    for (light, color) in colors {
      let on_light = self.declarations.entry(light.clone()).or_default();
      let order = match on_light.get(&id) {
        Some(existing) => existing.order,
        None => {
          self.next_order += 1;
          self.next_order
        }
      };
      let declaration = StatefulLightDeclaration {
        active,
        color: *color,
        order,
        mode,
      };
      on_light.insert(id, declaration);
    }
  }

  /// The color each light shows this frame, from the declarations and the playbacks shown. `None` for one nothing gives
  /// a color.
  pub(crate) fn resolve(&mut self, ctx: &SystemContext) -> Vec<(LightAddress, Option<Rgba<u8>>)> {
    self.draw_playbacks(ctx);
    let flash_on = self.flash_on();
    let undeclared = HashMap::new();
    let mut resolved = Vec::with_capacity(self.all_lights.len());
    for light in self.all_lights.iter() {
      let declarations = self.declarations.get(light);
      let played = self.played.get(light).map_or(&[][..], Vec::as_slice);
      if declarations.is_none() && played.is_empty() {
        log::trace!(target: "frontbox::lighting", "Nothing gives a color to light at {:?}", light);
        resolved.push((light.clone(), None));
        continue;
      }
      let declarations = declarations.unwrap_or(&undeclared);

      // every z with something on it for this light
      let z_indexes = declarations
        .iter()
        .filter(|(_, d)| d.shows(flash_on))
        .map(|(id, _)| id.z_index)
        .chain(played.iter().map(|(z, _)| *z))
        .sorted()
        .unique()
        .collect();

      let final_color = Self::resolve_light_color(
        light,
        z_indexes,
        declarations,
        played,
        flash_on,
        &self.conflict_resolution,
        &mut self.alternate_resolver,
      );
      resolved.push((light.clone(), Some(final_color)));
    }
    resolved
  }

  /// Samples the light layers of each playback the `SceneSystem` shows, into `played`
  fn draw_playbacks(&mut self, ctx: &SystemContext) {
    self.played.clear();
    let Some(scenes) = ctx.systems.get::<SceneSystem>() else {
      self.placements.clear();
      return;
    };
    let mut shown = HashSet::new();
    for (id, playback) in scenes.shown() {
      shown.insert(id);
      let placement = self.placements.entry(id).or_insert_with(|| place(playback.scene(), ctx));
      placement.update_visits(playback.scene(), &playback.moment());
      for (z, light, color) in placement.colors(playback.scene(), &playback.moment()) {
        if color[3] > 0 {
          self.played.entry(light.address.clone()).or_default().push((z, color));
        }
      }
    }
    self.placements.retain(|id, _| shown.contains(id));
  }

  fn resolve_light_color(
    light: &LightAddress,
    mut z_indexes: Vec<i8>,
    declarations: &HashMap<DeclarationIdentifier, StatefulLightDeclaration>,
    played: &[(i8, Rgba<u8>)],
    flash_on: bool,
    conflict_resolution: &HashMap<LightAddress, LightConflictResolution>,
    alternate_resolver: &mut AlternateResolver,
  ) -> Rgba<u8> {
    let max_z = z_indexes.iter().max().unwrap_or(&i8::MIN);
    let top_declarations = declarations
      .iter()
      .filter(|(id, dec)| id.z_index == *max_z && dec.shows(flash_on))
      .sorted_by_key(|(_, dec)| dec.order)
      .collect::<Vec<_>>();

    let top_color = if top_declarations.len() == 1 {
      top_declarations[0].1.color
    } else if top_declarations.len() > 1 {
      let resolution_strategy = conflict_resolution
        .get(light)
        .unwrap_or(&LightConflictResolution::FirstWins);

      log::debug!(
        target: "frontbox::lighting",
        "Resolving light declaration conflict on {:?} with {:?}",
        light,
        resolution_strategy
      );

      match resolution_strategy {
        LightConflictResolution::FirstWins => top_declarations[0].1.color,
        LightConflictResolution::Mix => top_declarations
          .iter()
          .map(|(_, d)| d.color)
          .reduce(|acc, c| acc.mix_with(c, 0.5))
          .unwrap_or(Rgba::default()),
        LightConflictResolution::Alternate => {
          let colors = top_declarations.iter().map(|(_, d)| d.color).collect();
          alternate_resolver.resolve(light.clone(), colors)
        }
      }
    } else {
      Rgba::default()
    };

    // playbacks on this z are drawn over its declarations, in the order they started
    let top_color = played.iter().filter(|(z, _)| z == max_z).fold(top_color, |below, (_, color)| color.composite_over(below));

    // if the top color is transparent, composite it with the next highest declaration below it
    if top_color.alpha() < 255 && z_indexes.len() > 1 {
      z_indexes.pop();
      let next_color = Self::resolve_light_color(
        light,
        z_indexes,
        declarations,
        played,
        flash_on,
        conflict_resolution,
        alternate_resolver,
      );
      top_color.composite_over(next_color)
    } else {
      top_color
    }
  }
}

impl LightManager for LayeredLightManager {
  fn declared_colors(&self, light: &LightAddress) -> Vec<Rgba<u8>> {
    self.declarations_for(light).iter().map(|d| d.color).collect()
  }

  fn declare(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], mode: LightMode) {
    self.declare_inner(owner, z, colors, mode, true);
  }

  fn declare_inactive(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], mode: LightMode) {
    self.declare_inner(owner, z, colors, mode, false);
  }

  fn undeclare(&mut self, system_id: u64, z: i8, lights: &[LightAddress]) {
    let target_id = DeclarationIdentifier { system_id, z_index: z };
    for light in lights {
      if let Some(declarations) = self.declarations.get_mut(light) {
        declarations.retain(|id, _| id != &target_id);
        if declarations.is_empty() {
          self.declarations.remove(light);
        }
      }
    }
  }

  fn undeclare_by_system(&mut self, system_id: u64) {
    log::debug!(target: "frontbox::lighting", "Removing all light declarations for {}", system_id);
    for declarations in self.declarations.values_mut() {
      declarations.retain(|id, _| id.system_id != system_id);
    }
  }

  fn deactivate_by_system(&mut self, system_id: u64) {
    log::debug!(target: "frontbox::lighting", "Deactivating all light declarations for {}", system_id);
    for declarations in self.declarations.values_mut() {
      for (id, declaration) in declarations.iter_mut() {
        if id.system_id == system_id {
          declaration.active = false;
        }
      }
    }
  }

  fn activate_by_system(&mut self, system_id: u64) {
    for declarations in self.declarations.values_mut() {
      for (id, declaration) in declarations.iter_mut() {
        if id.system_id == system_id {
          declaration.active = true;
        }
      }
    }
  }

  fn set_conflict_resolution(&mut self, lights: &[LightAddress], resolution: LightConflictResolution) {
    for light in lights {
      self.conflict_resolution.insert(light.clone(), resolution);
    }
  }
}

impl System for LayeredLightManager {
  fn provides(provides: &mut Provides<Self>) {
    provides.add::<dyn LightManager>(|s| s);
  }

  fn on_spawn(&mut self, ctx: &SystemContext) {
    // Create a copy of every light's address to reference during rendering
    self.all_lights = ctx.lights.values().map(|light| light.address.clone()).collect();
    self.thresholds = self
      .threshold_queries
      .iter()
      .flat_map(|(query, threshold)| query.resolve(ctx).into_iter().map(|light| (light, *threshold)))
      .collect();
  }

  fn on_event(&mut self, event: &dyn Event, _ctx: &SystemContext) {
    if let Some(SystemDespawned { id, .. }) = event.downcast_ref::<SystemDespawned>() {
      self.undeclare_by_system(*id);
    } else if let Some(SystemDeactivated { id, .. }) = event.downcast_ref::<SystemDeactivated>() {
      self.deactivate_by_system(*id);
    } else if let Some(SystemReactivated { id, .. }) = event.downcast_ref::<SystemReactivated>() {
      self.activate_by_system(*id);
    } else if event.is::<MachineReconnected>() {
      // the boards were reset, so send every light on the next render rather than only those that change
      self.prior_render.clear();
      self.prior_drivers.clear();
    }
  }

  fn on_tick(&mut self, delta: Duration, _ctx: &SystemContext) {
    self.alternate_resolver.accumulate(delta);
    self.flash_clock += delta;
    while self.flash_clock >= FLASH_PERIOD {
      self.flash_clock -= FLASH_PERIOD;
    }
  }

  fn on_render(&mut self, ctx: &SystemContext) {
    let Lowered { leds: mut leds_to_set, drivers } = lower(self.resolve(ctx), self.threshold, &self.thresholds);

    for (driver, on) in drivers {
      if self.prior_drivers.insert(driver, on) == Some(on) {
        continue;
      }
      log::debug!(target: "frontbox::lighting", "Turning driver light {driver} {}", if on { "on" } else { "off" });
      if on {
        ctx.drivers.activate(driver, ActivationMode::VirtualSwitchOn);
      } else {
        ctx.drivers.deactivate(driver, DeactivationMode::VirtualSwitchOff);
      }
    }

    // filter out LEDs that are the same as the prior render to avoid redundant updates
    // "there are hard limits to the amount of data that can go out over the EXP bus of about 80 characters per millisecond" -- ecurtz
    leds_to_set.retain(|(led, color)| {
      if let Some(prior_color) = self.prior_render.get(led) {
        if prior_color == color {
          return false;
        }
      }
      self.prior_render.insert(led.clone(), *color);
      log::debug!(target: "frontbox::lighting", "Setting LED at {:?} to {:?}", led, color);
      true
    });

    if leds_to_set.len() > 0 {
      // group by address to send to machine
      let outgoing: HashMap<ExpAddress, Vec<(u8, Rgb)>> =
        leds_to_set
          .into_iter()
          .fold(HashMap::new(), |mut acc, (addr, color)| {
            acc
              .entry(addr.exp)
              .or_insert_with(Vec::new)
              .push((addr.index, color));
            acc
          });

      for (address, leds) in outgoing.into_iter() {
        for chunk in leds.chunks(LED_SET_BATCH_SIZE) {
          ctx
            .lights
            .set(address, chunk.to_vec());
        }
      }
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct StatefulLightDeclaration {
  pub active: bool,
  pub color: Rgba<u8>,
  /// Counts up with each new declaration, and stays the same when a system changes the color of one it already made, so
  /// conflicts are resolved in the order systems first declared
  pub order: u64,
  pub mode: LightMode,
}

impl StatefulLightDeclaration {
  /// Whether it gives its light a color this frame: it's active, and if it flashes, the flash is on
  fn shows(&self, flash_on: bool) -> bool {
    self.active && (flash_on || self.mode == LightMode::Solid)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeclarationIdentifier {
  pub system_id: u64,
  pub z_index: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LightConflictResolution {
  /// Alternate between conflicting systems every N milliseconds (250ms)
  Alternate,
  /// Mix colors from conflicting systems, e.g. red and blue set on the same light renders as purple
  Mix,
  /// The first system to make a declaration wins (default)
  FirstWins,
}

/// Places a playback's scene on the lights, saying which of its layers can't reach any
fn place(scene: &Scene, ctx: &SystemContext) -> LightPlacement {
  for (index, layer) in scene.layers.iter().enumerate() {
    let Some(on) = layer.as_lights() else {
      continue;
    };
    if on.lights.is_none() {
      log::warn!(target: "frontbox::lighting", "{} of a scene has no lights, so it isn't shown", on.layer.label(index));
    }
    if let Some(plane) = on.surface.plane()
      && ctx.lights.plane(plane).is_none()
    {
      log::error!(target: "frontbox::lighting", "{} of a scene is on plane \"{plane}\", which wasn't added to the app", on.layer.label(index));
    }
  }
  LightPlacement::new(scene, &ctx.lights, ctx.light_radius)
}

/// What to write to each address for the colors its lights show. A light nothing gives a color is turned off, which
/// `on_render` leaves out if it's already off. Single-channel lights on the same LED address are written as one RGB.
fn lower(resolved: Vec<(LightAddress, Option<Rgba<u8>>)>, threshold: f32, thresholds: &HashMap<LightAddress, f32>) -> Lowered {
  let mut lowered = Lowered::default();
  // Where each LED address with single-channel lights is in `lowered.leds`
  let mut shared: HashMap<LedAddress, usize> = HashMap::new();
  for (light, color) in resolved {
    let color = color.map_or(Rgb::black(), led_color);
    match light {
      LightAddress::Exp { led, channels: LedChannels::Rgb } => lowered.leds.push((led, color)),
      LightAddress::Exp { led, channels: LedChannels::One(channel) } => {
        let index = *shared.entry(led.clone()).or_insert_with(|| {
          lowered.leds.push((led, Rgb::black()));
          lowered.leds.len() - 1
        });
        let value = color.r.max(color.g).max(color.b);
        let rgb = &mut lowered.leds[index].1;
        match channel {
          LedChannel::R => rgb.r = value,
          LedChannel::G => rgb.g = value,
          LedChannel::B => rgb.b = value,
        }
      }
      LightAddress::Driver { driver } => {
        let threshold = thresholds.get(&light).copied().unwrap_or(threshold);
        lowered.drivers.push((driver, brightness(color) >= threshold));
      }
    }
  }
  lowered
}

/// A color's brightness from 0 to 1: its largest channel
fn brightness(color: Rgb) -> f32 {
  color.r.max(color.g).max(color.b) as f32 / 255.0
}

/// One frame's lights, lowered to what's written to the hardware
#[derive(Debug, Default, PartialEq)]
struct Lowered {
  leds: Vec<(LedAddress, Rgb)>,
  /// Each driver light, and whether it's on
  drivers: Vec<(&'static str, bool)>,
}

/// The RGB an LED shows for a color, as if composited over an LED that's off: alpha dims it toward black
pub fn led_color(color: Rgba<u8>) -> Rgb {
  let [r, g, b, _] = color.composite_over(Rgba::black()).0;
  Rgb::new(r, g, b)
}

#[cfg(test)]
mod tests {
  use super::*;

  // Two different systems declare the same light
  #[test]
  fn declare_and_undeclare_systems() {
    let mut system = LayeredLightManager::new();
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    system.declare(42, 0, &[(light.clone(), Rgba::red())], LightMode::Solid);
    assert!(system.declarations.get(&light).unwrap().len() == 1);
    system.declare(43, 0, &[(light.clone(), Rgba::red())], LightMode::Solid);
    assert!(system.declarations.get(&light).unwrap().len() == 2);

    system.undeclare(42, 0, &[light.clone()]);

    // the declaration for system 43 should still be there
    assert!(system.declarations.get(&light).unwrap().len() == 1);
  }

  // One system declares multiple times to overwrite
  #[test]
  fn declare_overwrite() {
    let mut system = LayeredLightManager::new();
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    system.declare(42, 0, &[(light.clone(), Rgba::red())], LightMode::Solid);
    assert_eq!(system.declarations.get(&light).unwrap().len(), 1);
    system.declare(42, 0, &[(light.clone(), Rgba::blue())], LightMode::Solid);

    let final_map = system.declarations.get(&light).unwrap();
    assert_eq!(final_map.len(), 1);

    let final_dec = final_map.values().into_iter().next().unwrap();
    assert_eq!(final_dec.color, Rgba::blue());
  }

  // One system declares two different lights
  #[test]
  fn declare_and_undeclare_multiple() {
    let mut system = LayeredLightManager::new();
    let light1 = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });
    let light2 = LightAddress::rgb(LedAddress {
      exp: ExpAddress {
        board_address: 3,
        breakout: None,
      },
      port: 0,
      index: 2,
    });

    system.declare(42, 0, &[(light1.clone(), Rgba::red())], LightMode::Solid);
    system.declare(42, 0, &[(light2.clone(), Rgba::red())], LightMode::Solid);
    assert_eq!(system.declarations.get(&light1).unwrap().len(), 1);
    assert_eq!(system.declarations.get(&light2).unwrap().len(), 1);

    system.undeclare(42, 0, &[light1.clone()]);

    // the declaration for light2 should still be there
    assert_eq!(system.declarations.get(&light2).unwrap().len(), 1);
  }

  // One system declares the same light at two different z-indexes
  #[test]
  fn declare_and_undeclare_z_index() {
    let mut system = LayeredLightManager::new();
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    system.declare(42, 1, &[(light.clone(), Rgba::red())], LightMode::Solid);
    system.declare(42, 2, &[(light.clone(), Rgba::blue())], LightMode::Solid);
    assert!(system.declarations.get(&light).unwrap().len() == 2);

    system.undeclare(42, 1, &[light.clone()]);

    // the declaration for z-index 2 should still be there
    assert!(system.declarations.get(&light).unwrap().len() == 1);
  }

  #[test]
  fn alpha_fall_through_ignores_disabled() {
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 1,
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba::default(),
          order: 1,
          mode: LightMode::Solid,
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 0,
        },
        StatefulLightDeclaration {
          active: false, // final color should NOT be red
          color: Rgba::red(),
          order: 2,
          mode: LightMode::Solid,
        },
      ),
    ]);

    let conflict_resolution = HashMap::new();
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LayeredLightManager::resolve_light_color(
      &light,
      vec![0, 1],
      &declarations,
      &[],
      true,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    // final color should be clear since system 11 declarations are inactive
    assert_eq!(resolved_color, Rgba::default());
  }

  #[test]
  fn resolve_color() {
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 0,
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba::red(),
          order: 3,
          mode: LightMode::Solid,
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 1, // higher index, blue should win
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba::blue(),
          order: 4,
          mode: LightMode::Solid,
        },
      ),
    ]);

    let conflict_resolution = HashMap::new();
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LayeredLightManager::resolve_light_color(
      &light,
      vec![1],
      &declarations,
      &[],
      true,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    assert_eq!(resolved_color, Rgba([0, 0, 255, 255]));
  }

  #[test]
  fn resolve_color_conflict() {
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 0,
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba::red(),
          order: 5,
          mode: LightMode::Solid,
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 0,
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba::blue(),
          order: 6,
          mode: LightMode::Solid,
        },
      ),
    ]);

    let conflict_resolution = HashMap::from([(
      light.clone(),
      LightConflictResolution::Mix, // red and blue should mix to purple
    )]);
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LayeredLightManager::resolve_light_color(
      &light,
      vec![0],
      &declarations,
      &[],
      true,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    assert_eq!(resolved_color, Rgba([127, 0, 127, 255]));
  }

  // alpha compositing test - top semi-transparent declaration should composite with the next highest declaration below it
  #[test]
  fn resolve_color_alpha_compositing() {
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    let declarations = HashMap::from([
      (
        DeclarationIdentifier {
          system_id: 42,
          z_index: 1,
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba([255, 0, 0, 127]), // semi-transparent red
          order: 7,
          mode: LightMode::Solid,
        },
      ),
      (
        DeclarationIdentifier {
          system_id: 43,
          z_index: 0,
        },
        StatefulLightDeclaration {
          active: true,
          color: Rgba([255, 255, 255, 255]), // opaque white
          order: 8,
          mode: LightMode::Solid,
        },
      ),
    ]);

    let conflict_resolution = HashMap::from([(
      light.clone(),
      LightConflictResolution::Mix, // red and blue should mix to purple
    )]);
    let mut alternate_resolver = AlternateResolver::new();

    let resolved_color = LayeredLightManager::resolve_light_color(
      &light,
      vec![0, 1],
      &declarations,
      &[],
      true,
      &conflict_resolution,
      &mut alternate_resolver,
    );

    // final color shows through as pink
    assert_eq!(resolved_color, Rgba([255, 127, 127, 255]));
  }

  // A system's declarations stop rendering while it's inactive, and come back when it's reactivated
  #[test]
  fn follows_system_activity() {
    let ctx = TestContext::default();
    let mut system = LayeredLightManager::new();
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });
    system.declare(42, 0, &[(light.clone(), Rgba::red())], LightMode::Solid);
    let is_active = |system: &LayeredLightManager| system.declarations[&light].values().all(|d| d.active);

    system.on_event(&SystemDeactivated::new(42, ROOT_GROUP), &ctx.sys_ctx());
    assert!(!is_active(&system));

    system.on_event(&SystemReactivated::new(42, ROOT_GROUP), &ctx.sys_ctx());
    assert!(is_active(&system));
  }

  // Declaring from a system's context reaches the manager by its trait
  #[test]
  fn is_found_as_the_light_manager() {
    let mut ctx = TestContext::default();
    ctx.insert_system(LayeredLightManager::new());
    let light = LightAddress::rgb(LedAddress {
      exp: ExpAddress::new(3, None),
      port: 0,
      index: 1,
    });

    ctx.sys_ctx().declare_lights(&vec![light.clone()], 0, Rgba::red());

    let colors = ctx.sys_ctx().light_manager().unwrap().declared_colors(&light);
    assert_eq!(colors, vec![Rgba::red()]);
  }

  /// Which color wins on one light when systems 1 to `systems` declare on the same z in that order, and then the first changes
  /// its color
  fn first_wins_of(systems: u64) -> Rgba<u8> {
    let light = LightAddress::rgb(LedAddress::new(ExpAddress::new(3, None), 0, 1));
    let mut system = LayeredLightManager::new();
    for id in 1..=systems {
      system.declare(id, 0, &[(light.clone(), Rgba([id as u8, 0, 0, 255]))], LightMode::Solid);
    }
    system.declare(1, 0, &[(light.clone(), Rgba::blue())], LightMode::Solid);
    LayeredLightManager::resolve_light_color(&light, vec![0], &system.declarations[&light], &[], true, &HashMap::new(), &mut AlternateResolver::new())
  }

  #[test]
  fn first_wins_is_the_first_to_declare_even_once_it_changes_its_color() {
    // enough systems that a hash map's order would put another first
    assert_eq!(first_wins_of(32), Rgba::blue());
  }

  #[test]
  fn a_declaration_made_again_after_its_given_up_goes_to_the_back() {
    let light = LightAddress::rgb(LedAddress::new(ExpAddress::new(3, None), 0, 1));
    let mut system = LayeredLightManager::new();
    system.declare(1, 0, &[(light.clone(), Rgba::red())], LightMode::Solid);
    system.declare(2, 0, &[(light.clone(), Rgba::blue())], LightMode::Solid);
    system.undeclare(1, 0, &[light.clone()]);
    system.declare(1, 0, &[(light.clone(), Rgba::red())], LightMode::Solid);
    let color = LayeredLightManager::resolve_light_color(&light, vec![0], &system.declarations[&light], &[], true, &HashMap::new(), &mut AlternateResolver::new());
    assert_eq!(color, Rgba::blue());
  }

  #[test]
  fn a_flashing_declaration_shows_through_to_the_layer_below_while_its_off() {
    let light = LightAddress::rgb(LedAddress::new(ExpAddress::new(3, None), 0, 1));
    let mut system = LayeredLightManager::new();
    system.declare(1, 0, &[(light.clone(), Rgba::blue())], LightMode::Solid);
    system.declare(2, 1, &[(light.clone(), Rgba::green())], LightMode::Flashing);
    let color = |flash_on: bool, system: &LayeredLightManager| {
      let z_indexes = system.declarations[&light].iter().filter(|(_, d)| d.shows(flash_on)).map(|(id, _)| id.z_index).sorted().collect();
      LayeredLightManager::resolve_light_color(&light, z_indexes, &system.declarations[&light], &[], flash_on, &HashMap::new(), &mut AlternateResolver::new())
    };
    assert_eq!(color(true, &system), Rgba::green());
    assert_eq!(color(false, &system), Rgba::blue());
  }

  #[test]
  fn the_flash_is_on_for_the_first_half_of_each_period() {
    let mut system = LayeredLightManager::new();
    assert!(system.flash_on());
    system.flash_clock = FLASH_PERIOD / 2;
    assert!(!system.flash_on());
  }

  #[test]
  fn an_rgb_light_keeps_opaque_colors() {
    let led = LedAddress::new(ExpAddress::new(3, None), 0, 1);
    let lowered = lower(vec![(LightAddress::rgb(led.clone()), Some(Rgba([255, 128, 0, 255])))], DEFAULT_THRESHOLD, &HashMap::new());
    assert_eq!(lowered.leds, vec![(led, Rgb::new(255, 128, 0))]);
  }

  #[test]
  fn single_channel_lights_share_their_led_address() {
    let led = LedAddress::new(ExpAddress::new(3, None), 0, 1);
    let one = |channel| LightAddress::Exp { led: led.clone(), channels: LedChannels::One(channel) };
    let lowered = lower(
      vec![
        (one(LedChannel::R), Some(Rgba([0, 200, 0, 255]))),
        (one(LedChannel::B), Some(Rgba([255, 0, 0, 128]))),
      ],
      DEFAULT_THRESHOLD,
      &HashMap::new(),
    );
    assert_eq!(lowered.leds, vec![(led, Rgb::new(200, 0, 128))], "each writes its brightness into its channel");
  }

  #[test]
  fn an_rgb_light_dims_translucent_colors_toward_off() {
    assert_eq!(led_color(Rgba([255, 0, 200, 128])), Rgb::new(128, 0, 100));
    assert_eq!(led_color(Rgba::transparent()), Rgb::black());
  }

  #[test]
  fn a_light_nothing_gives_a_color_is_turned_off() {
    let led = LedAddress::new(ExpAddress::new(3, None), 0, 1);
    assert_eq!(lower(vec![(LightAddress::rgb(led.clone()), None)], DEFAULT_THRESHOLD, &HashMap::new()).leds, vec![(led, Rgb::black())]);
  }

  /// Whether a driver light lowered from `color` is on, with the default threshold or `threshold` for it
  fn driver_on(color: Option<Rgba<u8>>, threshold: Option<f32>) -> bool {
    let light = LightAddress::Driver { driver: "lamp" };
    let thresholds = threshold.map(|t| (light.clone(), t)).into_iter().collect();
    let lowered = lower(vec![(light, color)], DEFAULT_THRESHOLD, &thresholds);
    assert!(lowered.leds.is_empty());
    lowered.drivers == [("lamp", true)]
  }

  #[test]
  fn a_driver_light_is_on_at_or_above_its_threshold() {
    assert!(driver_on(Some(Rgba::white()), None));
    assert!(driver_on(Some(Rgba([0, 128, 0, 255])), None), "the largest channel is its brightness");
    assert!(!driver_on(Some(Rgba([100, 100, 100, 255])), None));
    assert!(!driver_on(None, None));
  }

  #[test]
  fn alpha_dims_a_driver_light() {
    assert!(!driver_on(Some(Rgba::white().with_alpha_f32(0.25)), None));
  }

  #[test]
  fn a_driver_light_can_have_its_own_threshold() {
    assert!(driver_on(Some(Rgba([100, 100, 100, 255])), Some(0.2)));
    assert!(!driver_on(Some(Rgba::white()), Some(1.1)));
  }
}
