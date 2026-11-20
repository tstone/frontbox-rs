use frontbox_core::prelude::*;
use frontbox_gfx::prelude::*;

use crate::*;

/// Light management is the single owner of the lights. It's the only thing that writes them, and every other system,
/// and every scene, declares through it.
///
/// Other systems find it by this trait, `ctx.systems.get_dyn::<dyn LightManager>()` or `ctx.light_manager()`, so an
/// implementation lists it in [`System::provides`]. [`LayeredLightManager`] is the one this crate provides.
///
/// ## Responsibilities
/// - **Holding declarations.** A declaration is a system asking for a color on some lights. It's keyed by the
///   declaring system and its z, so a system can't contradict itself: declaring again on the same lights and z replaces
///   what it declared before. Declarations stay until undeclared, or until their system despawns.
/// - **Following system activation.** A system's declarations stop showing while it's inactive and come back when it's
///   reactivated, without the system doing anything.
/// - **Playing scenes.** It shows the light layers of the scenes the [`SceneSystem`](frontbox_scene::SceneSystem) is
///   playing, alongside the declarations.
/// - **Deciding what each light shows**, from everything declared and played on it, and writing only what changed.
///
/// ## Not responsible for
/// - Which scenes play. That's the `SceneSystem`.
/// - What a light means to the game. Systems declare what they want shown; light management only shows it.
pub trait LightManager: SpawnableSystem {
  /// Declares a color on each light for `owner` (its `ctx.id()`) at `z`, shown from the next render, solid or flashing
  /// by `mode`.
  fn declare(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], mode: LightMode);
  /// Declares like [`Self::declare`], but not shown until `activate_by_system`. Lets a system prepare declarations in
  /// advance and show them all at once.
  fn declare_inactive(&mut self, owner: u64, z: i8, colors: &[(LightAddress, Rgba<u8>)], mode: LightMode);
  /// Removes what `owner` declared on these lights at `z`
  fn undeclare(&mut self, owner: u64, z: i8, lights: &[LightAddress]);
  /// Removes everything `owner` declared
  fn undeclare_by_system(&mut self, owner: u64);
  /// Shows `owner`'s declarations again
  fn activate_by_system(&mut self, owner: u64);
  /// Keeps `owner`'s declarations but stops showing them
  fn deactivate_by_system(&mut self, owner: u64);
  /// How these lights choose between declarations from different systems at the same z
  fn set_conflict_resolution(&mut self, lights: &[LightAddress], resolution: LightConflictResolution);
  /// The colors declared for a light by any system, in no particular order
  fn declared_colors(&self, light: &LightAddress) -> Vec<Rgba<u8>>;
}

/// How a declared color shows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LightMode {
  /// On the whole time
  Solid,
  /// On and off, in step with every other flashing declaration, once each [`FLASH_PERIOD`]
  Flashing,
}
