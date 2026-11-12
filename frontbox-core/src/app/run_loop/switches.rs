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

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use std::rc::Rc;

  use super::*;
  use crate::app::run_loop::test_support::*;

  const START: usize = 0;

  /// A harness whose machine has one switch, `start`, open
  fn with_start_switch() -> Harness {
    let definition: &'static SwitchDefinition = Box::leak(Box::new(SwitchDefinition::new("start").build()));
    let start = IoAddressed { definition, assignment: IoAddress::new(0, 0), id: START };
    let switches = SwitchLookup::new(vec![start], vec![]);
    Harness::with_state(AppState { switches, ..AppState::default() })
  }

  /// Records, when it hears `start` close, whether the context already reads it as closed
  struct ReadsStart(Rc<RefCell<Vec<Option<bool>>>>);

  impl System for ReadsStart {
    fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
      if event.is::<SwitchClosed>() {
        self.0.borrow_mut().push(ctx.switches.is_closed("start"));
      }
    }
  }

  #[test]
  fn a_switch_change_is_announced_after_its_state_is_updated() {
    let mut h = with_start_switch();
    let heard = h.listen();
    let read = Rc::new(RefCell::new(Vec::new()));
    h.spawn(ReadsStart(read.clone()));
    heard.take();

    h.send(AppMessage::SwitchStateChange(START, SwitchState::Closed));
    h.send(AppMessage::SwitchStateChange(START, SwitchState::Open));
    h.settle();

    assert_eq!(heard.take(), [Heard::SwitchClosed("start"), Heard::SwitchOpened("start")]);
    assert_eq!(*read.borrow(), [Some(true)]);
  }

  #[test]
  fn an_unknown_switch_is_not_announced() {
    let mut h = with_start_switch();
    let heard = h.listen();

    h.send(AppMessage::SwitchStateChange(99, SwitchState::Closed));
    h.settle();

    assert_eq!(heard.take(), []);
  }

  #[test]
  fn syncing_switch_states_updates_them_without_announcing() {
    let mut h = with_start_switch();
    let heard = h.listen();

    h.send(AppMessage::SyncSwitchStates(vec![SwitchState::Closed]));
    h.settle();

    assert_eq!(h.base.switches.is_closed_by_id(START), Some(true));
    assert_eq!(heard.take(), []);
  }

  #[test]
  fn a_tracer_can_close_an_open_switch_and_open_a_closed_one() {
    let mut h = with_start_switch();
    let heard = h.listen();

    h.handle_tracer_control(TracerControlEvent::OpenSwitch { switch_id: START });
    h.handle_tracer_control(TracerControlEvent::CloseSwitch { switch_id: START });
    h.handle_tracer_control(TracerControlEvent::CloseSwitch { switch_id: START });
    h.handle_tracer_control(TracerControlEvent::OpenSwitch { switch_id: START });
    h.settle();

    // it starts open, so the first open and the second close do nothing
    assert_eq!(heard.take(), [Heard::SwitchClosed("start"), Heard::SwitchOpened("start")]);
  }
}
