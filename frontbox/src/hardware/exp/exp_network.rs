use crate::hardware::ExpBoard;

/// The expansion network is defined using the `ExpansionNetworkBuilder`. See [Defining Hardware]() guide for more details.
/// 
/// ```rust
/// // Step 1. Define exp devices
/// 
/// pub mod leds {
///   use super::*;
/// 
///   hardware_defs! {
///     pub LEFT_INLANE: LedDefinition = LedDefinition::single("linlane")
///       .location(Vec2::new(3.4, 32.5).relative_to(PLAYFIELD))
///       .channels(ColorChannels::GRBW);
/// 
///     pub LEFT_OUTLANE: LedDefinition = LedDefinition::new("loutlane")
///       .location(Vec2::new(2.125, 32.5).relative_to(PLAYFIELD));
/// 
///     // Cabinet lighting along the left art blade area
///     pub LEFT_CAB_STRIP: LedDefinition = LedDefinition::strip("lcab", 32)
///       .tag(Cabinet)
///       .locations(CabinetLeft, 15.0, (10.25, 0), (48.5, 3.0));
///   }
/// }
/// 
/// // Step 2. Define boards and wire the network
/// 
/// let exp_network = ExpansionNetwork::new(vec![
///   ExpansionBoard::fp_exp0061()
///     .wire_led_port(0, LedPort::ws2812b().leds(vec![
///         &leds::LEFT_INLANE,
///         &leds::LEFT_OUTLANE,
///       ]
///     )),
///     .wire_led_port(1, LedPort::wS2812b().leds(vec![&leds::LEFT_CAB_STRIP]))
/// ]);
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
