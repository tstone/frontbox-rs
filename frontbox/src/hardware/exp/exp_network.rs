use crate::hardware::ExpBoard;

/// The expansion network is defined using `ExpNetwork`. See [Defining Hardware](mod@crate::hardware) guide for more details.
/// 
/// ```rust
/// # use frontbox::prelude::*;
/// # use frontbox::tags::*;
/// # static PLAYFIELD: ReferencePlane = ReferencePlane::new("Playfield")
/// #   .origin(Vec3::new(1.0, 3.25, 12.0))
/// #   .extent(Vec2::new(20.25, 45.0))
/// #   .build();
/// # static CABINET_LEFT: ReferencePlane = ReferencePlane::new("Cabinet left")
/// #   .origin(Vec3::new(10.25, 0.0, 0.0))
/// #   .extent(Vec2::new(48.5, 3.0))
/// #   .build();
/// // Step 1. Define exp devices
/// 
/// pub mod leds {
///   use super::*;
/// 
///   hardware_defs! {
///     pub LEFT_INLANE: LedDefinition = LedDefinition::single("linlane")
///       .location(Vec2::new(3.4, 32.5).relative_to(&PLAYFIELD))
///       .channels(LedChannels::GRBW);
/// 
///     pub LEFT_OUTLANE: LedDefinition = LedDefinition::single("loutlane")
///       .location(Vec2::new(2.125, 32.5).relative_to(&PLAYFIELD));
/// 
///     // Cabinet lighting along the left art blade area
///     pub LEFT_CAB_STRIP: LedDefinition = LedDefinition::strip("lcab", 32)
///       .tag(Cabinet)
///       .locations(&CABINET_LEFT, LedStripDirection::Forwards);
///   }
/// }
/// 
/// # fn main() {
/// // Step 2. Define boards and wire the network
/// 
/// let exp_network = ExpNetwork::new(vec![
///   ExpBoard::fp_exp0061(JumperState::Open, JumperState::Open)
///     .wire_led_port(1, LedPort::ws2812().leds(vec![
///       &leds::LEFT_INLANE,
///       &leds::LEFT_OUTLANE,
///     ]))
///     .wire_led_port(2, LedPort::ws2812().leds(vec![&leds::LEFT_CAB_STRIP]))
/// ]);
/// # }
/// ```
#[derive(Default)]
pub struct ExpNetwork {
  pub boards: Vec<ExpBoard>
}

impl ExpNetwork {
  pub fn new(boards: Vec<ExpBoard>) -> Self {
    Self { boards }
  }

  pub fn empty() -> Self {
    Self::new(Vec::new())
  }
}
