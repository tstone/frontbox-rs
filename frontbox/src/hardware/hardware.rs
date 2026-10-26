use std::collections::HashMap;

use crate::prelude::*;

/// FAST EXP boards address LEDs per breakout in a single byte, `0x00`-`0x7F`
const MAX_LEDS_PER_BREAKOUT: usize = 128;

#[derive(Clone, serde::Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Hardware {
  pub switches: SwitchLookup,
  pub drivers: DriverLookup,
  pub leds: LedLookup,
  pub io_network: Vec<ResolvedIoBoard>,
  pub exp_network: Vec<ResolvedExpansionBoard>,
}

impl Hardware {
  pub fn new(
    switches: SwitchLookup,
    drivers: DriverLookup,
    leds: LedLookup,
    io_network: Vec<ResolvedIoBoard>,
    exp_network: Vec<ResolvedExpansionBoard>,
  ) -> Self {
    // verify switches associated with driver configs actually exist
    let unknown = Self::unknown_switch_references(&drivers, &switches);
    if !unknown.is_empty() {
      let list = unknown
        .iter()
        .map(|(driver, switch)| format!("  driver \"{}\" -> switch \"{}\"", driver, switch))
        .collect::<Vec<_>>()
        .join("\n");
      panic!(
        "Driver modes reference switches that don't exist:\n{}",
        list
      );
    }

    let duplicate_configs = Self::duplicate_config_keys(&drivers);
    if !duplicate_configs.is_empty() {
      let list = duplicate_configs
        .iter()
        .map(|(key, first, second)| {
          format!("  \"{}\" on drivers \"{}\" and \"{}\"", key, first, second)
        })
        .collect::<Vec<_>>()
        .join("\n");
      panic!(
        "Hardware config keys must be unique; give one of each a distinct `.key(...)`:\n{}",
        list
      );
    }

    Self {
      switches,
      drivers,
      leds,
      io_network,
      exp_network,
    }
  }

  /// Take the user-defined expansion board configurations and resolve actual hardware indexes/addresses
  pub fn resolve_expansion_boards(expansion_boards: &Vec<ExpBoard>) -> Vec<ResolvedExpansionBoard> {
    let mut resolved_boards = Vec::new();
    for board in expansion_boards {
      if board.model == FastExpansionBoardModels::Neuron {
        log::warn!(
          "Remapping Neuron LED ports to default 32-LED configuration for backwards compatibility"
        );
      }

      // Known bug: Older versions of the Neuron have an internal Exp board which does not respect ER:
      let port_count_override = match board.model {
        FastExpansionBoardModels::Neuron => Some(32),
        _ => None,
      };

      // sum up actual LEDs present and calculate index offsets
      let mut offset: u8 = 0;
      let mut resolved_ports = Vec::new();
      for port_idx in 0..board.hardware_led_port_count.unwrap_or(0) {
        // For EXP boards which have multiple LED port banks, the offset needs to be reset every 5th port
        if port_idx > 0 && port_idx % 4 == 0 {
          offset = 0;
        }

        let port = if let Some(port) = board.led_ports.get(&port_idx) {
          Self::resolve_led_port(board, port, port_idx, offset, port_count_override)
        } else {
          // No port defined. The Neuron's EXP LED ports stay at their fixed length, so take that or whatever room is left on the breakout.
          let length = match port_count_override {
            Some(length) => (MAX_LEDS_PER_BREAKOUT - offset as usize).min(length as usize) as u8,
            None => 0,
          };
          ResolvedLedPort {
            length,
            ..ResolvedLedPort::default(offset)
          }
        };

        let breakout_end = offset as usize + port.length as usize;
        if breakout_end > MAX_LEDS_PER_BREAKOUT {
          panic!(
            "LED port {} on board {:X} ends at LED {}, but a breakout only has {} LEDs",
            port_idx + 1,
            board.address,
            breakout_end,
            MAX_LEDS_PER_BREAKOUT
          );
        }
        offset = breakout_end as u8;
        resolved_ports.push(port);
      }

      resolved_boards.push(ResolvedExpansionBoard {
        address: board.address,
        breakout: board.breakout,
        led_ports: resolved_ports,
        model: board.model,
      });
    }
    resolved_boards
  }

  fn resolve_led_port(
    board: &ExpBoard,
    port: &LedPort,
    port_idx: u8,
    offset: u8,
    port_count_override: Option<u8>,
  ) -> ResolvedLedPort {
    let mut port_led_total_count: usize = 0;
    let mut addressed_leds = Vec::new();
    for multi_def in &port.leds {
      multi_def
        .children()
        .iter()
        .enumerate()
        .for_each(|(i, led)| {
          addressed_leds.push(ExpAddressed {
            definition: led.clone(),
            assignment: ExpAddress {
              board_address: board.address,
              breakout: Some(port_idx / 4),
              port: port_idx as u8,
            },
            id: i + port_led_total_count + offset as usize,
          });
        });

      port_led_total_count += multi_def.children().len();
    }

    log::trace!("Mapped {} leds: {:?}", addressed_leds.len(), addressed_leds);

    // only ports too long for a u8 are caught here; resolve_expansion_boards checks the port fits on its breakout
    let length = match port_count_override {
      Some(length) if port_led_total_count > length as usize => panic!(
        "LED port {} on board {:X} has {} LEDs, but only supports {}",
        port_idx + 1,
        board.address,
        port_led_total_count,
        length
      ),
      Some(length) => length,
      None => u8::try_from(port_led_total_count).unwrap_or_else(|_| {
        panic!(
          "LED port {} on board {:X} has {} LEDs, but a breakout only has {} LEDs",
          port_idx + 1,
          board.address,
          port_led_total_count,
          MAX_LEDS_PER_BREAKOUT
        )
      }),
    };

    ResolvedLedPort {
      led_type: port.led_type.clone(),
      offset,
      length,
      leds: addressed_leds,
    }
  }

  /// (driver name, switch name) for every switch a driver mode references that isn't in `switches`
  fn unknown_switch_references(
    drivers: &DriverLookup,
    switches: &SwitchLookup,
  ) -> Vec<(&'static str, &'static str)> {
    let mut unknown = Vec::new();
    for driver in drivers.values() {
      if let Some(mode) = drivers.config(driver.name) {
        for switch in mode.switch_names() {
          if switches.by_name(switch).is_none() {
            unknown.push((driver.name, switch));
          }
        }
      }
    }
    unknown.sort();
    unknown
  }

  /// (key, first driver, second driver) for every config key used more than once across driver modes
  fn duplicate_config_keys(
    drivers: &DriverLookup,
  ) -> Vec<(&'static str, &'static str, &'static str)> {
    let mut drivers_by_name: Vec<_> = drivers.values().map(|driver| driver.name).collect();
    drivers_by_name.sort();

    let mut seen = HashMap::new();
    let mut duplicates = Vec::new();
    for name in drivers_by_name {
      let Some(mode) = drivers.config(name) else {
        continue;
      };
      for config in mode.configs() {
        if let Some(first) = seen.insert(config.key, name) {
          duplicates.push((config.key, first, name));
        }
      }
    }
    duplicates.sort();
    duplicates
  }
}

#[cfg(test)]
mod tests {
  use std::sync::LazyLock;

  use super::*;

  static SWITCH: LazyLock<SwitchDefinition> =
    LazyLock::new(|| SwitchDefinition::new("real").build());
  static GOOD: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("good")
      .mode(
        DriverMode::pulse()
          .trigger_mode(DriverTriggerMode::Switch("real"))
          .build(),
      )
      .build()
  });
  static TYPO: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("typo")
      .mode(
        DriverMode::pulse()
          .trigger_mode(DriverTriggerMode::Switch("reel"))
          .build(),
      )
      .build()
  });
  static FLIPPER: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("flipper")
      .mode(DriverMode::flipper_main_direct("real", "missing_eos").build())
      .build()
  });
  static VIRTUAL: LazyLock<DriverDefinition> = LazyLock::new(|| {
    DriverDefinition::new("virtual")
      .mode(DriverMode::pulse().build())
      .build()
  });

  fn addressed<T: HardwareDefinition>(defs: Vec<&'static T>) -> Vec<IoAddressed<T>> {
    defs
      .into_iter()
      .enumerate()
      .map(|(id, definition)| IoAddressed {
        definition,
        assignment: IoAddress::new(0, id as u16),
        id,
      })
      .collect()
  }

  #[test]
  fn finds_unknown_switch_references() {
    let switches = SwitchLookup::new(addressed(vec![&*SWITCH]), vec![]);
    let drivers = DriverLookup::new(addressed(vec![&*GOOD, &*TYPO, &*FLIPPER, &*VIRTUAL]));

    assert_eq!(
      Hardware::unknown_switch_references(&drivers, &switches),
      vec![("flipper", "missing_eos"), ("typo", "reel")]
    );
  }

  fn flipper_with_power(name: &'static str, power: HardwareValue<Power>) -> DriverDefinition {
    DriverDefinition::new(name)
      .mode(
        DriverMode::flipper_main_direct("real", "real")
          .initial_pwm_power(power)
          .build(),
      )
      .build()
  }

  fn power_config() -> HardwareValue<Power> {
    HardwareValue::config("Initial Power", "", Power::FULL, Ranges::full_power())
  }

  static LEFT: LazyLock<DriverDefinition> =
    LazyLock::new(|| flipper_with_power("left", power_config()));
  static RIGHT: LazyLock<DriverDefinition> =
    LazyLock::new(|| flipper_with_power("right", power_config()));
  static RIGHT_KEYED: LazyLock<DriverDefinition> =
    LazyLock::new(|| flipper_with_power("right", power_config().key("Right Initial Power")));

  #[test]
  fn identical_hardware_configs_on_one_key_are_duplicates() {
    let drivers = DriverLookup::new(addressed(vec![&*LEFT, &*RIGHT]));
    assert_eq!(
      Hardware::duplicate_config_keys(&drivers),
      vec![("Initial Power", "left", "right")]
    );
  }

  #[test]
  fn an_explicit_key_resolves_a_duplicate() {
    let drivers = DriverLookup::new(addressed(vec![&*LEFT, &*RIGHT_KEYED]));
    assert!(Hardware::duplicate_config_keys(&drivers).is_empty());
  }

  #[test]
  #[should_panic(expected = "Hardware config keys must be unique")]
  fn duplicate_hardware_config_keys_panic_at_boot() {
    let switches = SwitchLookup::new(addressed(vec![&*SWITCH]), vec![]);
    let drivers = DriverLookup::new(addressed(vec![&*LEFT, &*RIGHT]));
    Hardware::new(switches, drivers, LedLookup::default(), vec![], vec![]);
  }

  static STRIP_64: LazyLock<LedDefinition> =
    LazyLock::new(|| LedDefinition::multi("strip_64").count(64).build());
  static STRIP_65: LazyLock<LedDefinition> =
    LazyLock::new(|| LedDefinition::multi("strip_65").count(65).build());
  static STRIP_32: LazyLock<LedDefinition> =
    LazyLock::new(|| LedDefinition::multi("strip_32").count(32).build());

  fn exp_board() -> ExpBoard {
    ExpBoard::fp_exp0061(JumperState::Open, JumperState::Open)
  }

  fn layout(board: ExpBoard) -> Vec<(u8, u8)> {
    Hardware::resolve_expansion_boards(&vec![board])[0]
      .led_ports
      .iter()
      .map(|p| (p.offset, p.length))
      .collect()
  }

  #[test]
  fn undefined_ports_take_no_leds() {
    // An 8 port board is two breakouts, each indexed from 0. Without the unwired port 1 reserving 32 LEDs, ports
    // 2-4 fit in the first breakout.
    let board = ExpBoard::fp_exp0081(JumperState::Open, JumperState::Open)
      .wire_led_port(2, LedPort::ws2812().leds(vec![&*STRIP_64]))
      .wire_led_port(3, LedPort::ws2812().leds(vec![&*STRIP_32]))
      .wire_led_port(4, LedPort::ws2812().leds(vec![&*STRIP_32]))
      .wire_led_port(6, LedPort::ws2812().leds(vec![&*STRIP_32]));

    assert_eq!(
      layout(board),
      vec![
        (0, 0),
        (0, 64),
        (64, 32),
        (96, 32),
        (0, 0),
        (0, 32),
        (32, 0),
        (32, 0)
      ]
    );
  }

  #[test]
  fn undefined_neuron_ports_keep_fixed_length() {
    let board = ExpBoard::neuron().wire_led_port(2, LedPort::ws2812().leds(vec![&*STRIP_32]));

    assert_eq!(layout(board), vec![(0, 32), (32, 32), (64, 32), (96, 32)]);
  }

  #[test]
  #[should_panic(expected = "a breakout only has 128 LEDs")]
  fn panics_when_ports_overflow_the_breakout() {
    let board = exp_board()
      .wire_led_port(1, LedPort::ws2812().leds(vec![&*STRIP_64]))
      .wire_led_port(2, LedPort::ws2812().leds(vec![&*STRIP_65]));

    Hardware::resolve_expansion_boards(&vec![board]);
  }

  #[test]
  #[should_panic(expected = "only supports 32")]
  fn panics_when_neuron_port_exceeds_fixed_length() {
    let board = ExpBoard::neuron().wire_led_port(1, LedPort::ws2812().leds(vec![&*STRIP_64]));

    Hardware::resolve_expansion_boards(&vec![board]);
  }
}
