//! # Core Hardware
//! 
//! - drivers (coils)
//! - switches
//! - LEDs
//! 
//! ### Common Configuration & Identification
//! 
//! Every piece of core hardware shares three points of definition in common, which also represent 3 ways of describing hardware:
//! 
//! - **Name** - Name is a unique, static string identifier that will always refer to that exact piece of hardware. In most cases this is explicitly given, but in some cases (e.g. LED strips) it is automatically generated.
//! - **Tags** - Tags are just empty structs which are a way to arbitrarily classify something (e.g. on the playfield, part of a mech, of a type, etc.). Frontbox [includes a handful of tags](crate::hardware::tags), but these can be user-defined as well.
//! - **Location** - Location specifies where something is in space, and is typically used for LED canvas rendering or complex spatial effects.
//! 
//! Thus, in Frontbox hardware can typically always be referenced in these 3 ways: directly by it's name, or implicitly but it's tags or location. See [hardware querying](crate::hardware::Q) for details on how this works.
//! 
//! ```rust,no_run
//! Q::name("example")
//! Q::tag::<Example>()
//! ```
//!
//! ### Definition Phases
//! 
//! Configured hardware moves through four phases within Frontbox:
//!
//! 1. **Definition** -- Static configuration such as name, tags, location, and any configuration _(user responsibility)_
//! ```rust
//! static EXAMPLE: LazyLock<SwitchDefinition> = LazyLock::new(|| {
//!   SwitchDefinition::new("example_switch")
//!    .tag(Playfield)
//!    .tag(Target)
//!    .build()
//! });
//! ```
//! 
//! 2. **Wiring** -- A static hardware definition is assigned to a specific pin on a specific board _(user responsibility)_
//! ```rust
//! # static EXAMPLE: LazyLock<SwitchDefinition> = LazyLock::new(|| {
//! #  SwitchDefinition::new("example_switch")
//! #   .tag(Playfield)
//! #   .tag(Target)
//! # });
//! let io_network = IoNetwork::new(vec![
//!     IoBoards::io_3208()
//!      .wire_switch(0, &EXAMPLE)
//! ]);
//! ```
//! 
//! 3. **Addressable** -- A wired definition is automatically resolved, on boot, to it's absolute address (id) on the network _(framework responsibility)_
//! 4. **Stateful** -- Some hardware (e.g. switches) also become stateful, keeping track of things like open/closed state _(framework responsibility)_ (accessible via [Context](crate::systems::Context))
//! ```rust,no_run
//! # static EXAMPLE: LazyLock<SwitchDefinition> = LazyLock::new(|| {
//! #  SwitchDefinition::new("example_switch")
//! #   .tag(Playfield)
//! #   .tag(Target)
//! # });
//! fn on_spawn(&mut self, ctx: &Context) {
//!   if ctx.switches.is_open(EXAMPLE.name) {
//!     // do something
//!   }
//! }
//! ```
//! 
//! ### Defining Hardware Macro
//! 
//! For convinience, a `hardware_defs!`` macro simplifies the LazyLock setup. Use of this is optional, but does make definitions shorter to declare.
//! 
//! ```rust
//! hardware_defs! {
//!   pub SWITCH: SwitchDefinition = SwitchDefinition::new("example");
//!   pub LED_STRIP: LedDefinition = LedDefinition::multi("example", 12);
//! }
//! ```
//! 
//! ### Best Pratices
//! 
//! - Don't use one large file for everything
//! - Split up a hardware into multiple files, having either one file per mech or lane (e.g. `left_orbit.rs`, `right_ramp.rs`) or by region (e.g. `upper_playfield.rs`) ([Example](https://github.com/tstone/lotko-homebrew/tree/main/src/hardware))
//! - Group common query definitions into the hardware definition files
//! - Implement basic event wrapper systems for hardware into the same spot
//! 

mod exp;
mod fast_platform;
mod hardware;
mod hardware_definition;
mod hardware_query;
mod hardware_query_conversions;
mod io;
mod location;
mod q;

pub use exp::*;
pub use fast_platform::*;
pub use hardware::*;
pub use hardware_definition::*;
pub use hardware_query::*;
#[allow(unused)]
pub use hardware_query_conversions::*;
pub use io::*;
pub use location::*;
pub use q::*;
