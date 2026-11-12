use super::*;

impl RunLoop {
  pub(super) fn switch_state_changed(&mut self, sender: Option<u64>, switch_id: usize, state: SwitchState) {
    // Update this first so that handled events see the latest state in Context
    self.base.switches.update_switch_state(switch_id, state);

    let outgoing_event = match (self.base.switches.by_id(&switch_id), state) {
      (Some(switch), SwitchState::Closed) => {
        log::debug!(target: "frontbox_core::switches", "🎚️  Switch {} closed", switch.name);
        if let Some(hits) = self.base.switches.hits_counter(switch_id) {
          self.base.metrics.inc(hits);
        }
        Some(EventBox::new(SwitchClosed::new(switch.clone())))
      }
      (Some(switch), SwitchState::Open) => {
        log::debug!(target: "frontbox_core::switches", "🎚️  Switch {} opened", switch.name);
        Some(EventBox::new(SwitchOpened::new(switch.clone())))
      }
      (None, _) => {
        // Report as native board/switch id since this is the easiest way to figure out current switch wiring
        match get_native_switch_id(&switch_id, &self.base) {
          Some((board_id, local_id)) => {
            log::warn!(
              target: "frontbox_core::switches",
              "Received event for unknown switch -- board: {}, id: {} -- {:?}",
              board_id,
              local_id,
              state
            );
            return;
          }
          None => {
            log::warn!(
              target: "frontbox_core::switches",
              "Received event for unknown switch on unknown board {} -- {:?}",
              switch_id,
              state
            );
          }
        }
        None
      }
    };

    if let Some(event) = outgoing_event {
      self.emit_event(sender, event);
      self.tracers.send(TraceEvent::SwitchStateChange { switch_id, state });
    }
  }
}

fn get_native_switch_id(switch_id: &usize, base: &AppState) -> Option<(usize, usize)> {
  let mut offset: usize = 0;
  for (index, board) in base.io_network.iter().enumerate() {
    if *switch_id < (board.switch_count as usize) + offset {
      let native_switch_id = switch_id - offset;
      return Some((index, native_switch_id));
    }
    offset += board.switch_count as usize;
  }
  None
}
