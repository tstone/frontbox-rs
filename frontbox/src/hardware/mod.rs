//! # Core Hardware
//!
//! <div class="warning">Stability Level: Moderate-High</div>
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
//! Thus, in Frontbox hardware can typically always be referenced in these 3 ways: directly by it's name, or implicitly but it's tags or location. See hardware querying ([SwitchQ], [DriverQ], [LedQ]) for details on how this works.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! # #[derive(Tag)]
//! # struct Example;
//! SwitchQ::name("example");
//! SwitchQ::tag::<Example>();
//! ```
//!
//! ### Definition Phases
//!
//! Configured hardware moves through four phases within Frontbox:
//!
//! 1. **Definition** -- Static configuration such as name, tags, location, and any configuration _(user responsibility)_
//! ```rust
//! # use frontbox::prelude::*;
//! # use frontbox::tags::*;
//! # use std::sync::LazyLock;
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
//! # use frontbox::prelude::*;
//! # use frontbox::tags::*;
//! # use std::sync::LazyLock;
//! # static EXAMPLE: LazyLock<SwitchDefinition> = LazyLock::new(|| {
//! #  SwitchDefinition::new("example_switch")
//! #   .tag(Playfield)
//! #   .tag(Target)
//! #   .build()
//! # });
//! let io_network = IoNetwork::new(vec![
//!     IoBoards::io_3208()
//!      .wire_switch(0, &EXAMPLE)
//! ]);
//! ```
//!
//! 3. **Addressable** -- A wired definition is automatically resolved, on boot, to it's absolute address (id) on the network _(framework responsibility)_
//! 4. **Stateful** -- Some hardware (e.g. switches) also become stateful, keeping track of things like open/closed state _(framework responsibility)_ (accessible via [SystemContext](crate::systems::SystemContext))
//! ```rust
//! # use frontbox::prelude::*;
//! # use frontbox::tags::*;
//! # use std::sync::LazyLock;
//! # static EXAMPLE: LazyLock<SwitchDefinition> = LazyLock::new(|| {
//! #  SwitchDefinition::new("example_switch")
//! #   .tag(Playfield)
//! #   .tag(Target)
//! #   .build()
//! # });
//! # struct ExampleSystem;
//! # impl System for ExampleSystem {
//! fn on_spawn(&mut self, ctx: &SystemContext) {
//!   if ctx.switches.is_open(EXAMPLE.name).unwrap_or(false) {
//!     // do something
//!   }
//! }
//! # }
//! ```
//!
//! ### Defining Hardware Macro
//!
//! For convinience, a `hardware_defs!` macro simplifies the LazyLock setup. Use of this is optional, but does make definitions shorter to declare.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! hardware_defs! {
//!   pub SWITCH: SwitchDefinition = SwitchDefinition::new("example");
//!   pub LED_STRIP: LedDefinition = LedDefinition::multi("example").count(12);
//! }
//! ```
//!
//! ### Best Practices
//!
//! Despite the examples in this document showing everything defined all together, it usually makes more sense to group all definitions by region or mech, then wire the network in the same spot. For example, a game might be setup to have regions like `hardware/lower_thirds.rs`, `hardware/mid_field.rs`, and `hardware/upper_playfield.rs`, then a separate `io_network.rs` and `exp_network.rs` which references those hardware definitions and wires up the network.
//!
//! Depending on the complexity of the playfield, it might also be useful to group regions into modules.
//!
//! ```rust
//! # use frontbox::prelude::*;
//! // upper_playfield.rs
//!
//! hardware_defs! {
//!   pub LEFT_RAMP_SWITCH: SwitchDefinition = SwitchDefinition::new("left_ramp");
//!   pub RIGHT_RAMP_SWITCH: SwitchDefinition = SwitchDefinition::new("right_ramp");
//! }
//!
//! pub mod custom_mech {
//!   use super::*;
//!
//!   hardware_defs! {
//!     pub ENTRANCE_SWITCH: SwitchDefinition = SwitchDefinition::new("custom_mech_entrance");
//!     pub KICKER_COIL: DriverDefinition = DriverDefinition::new("custom_mech_kicker");
//!   }
//! }
//! # fn main() {}
//! ```

mod driver_query;
mod exp;
mod fast_platform;
mod hardware;
mod hardware_definition;
mod hardware_query_conversions;
mod io;
mod led_query;
mod region;
mod switch_query;

pub use driver_query::*;
pub use exp::*;
pub use fast_platform::*;
pub use hardware::*;
pub use hardware_definition::*;
#[allow(unused)]
pub use hardware_query_conversions::*;
pub use io::*;
pub use led_query::*;
pub use region::*;
pub use switch_query::*;
