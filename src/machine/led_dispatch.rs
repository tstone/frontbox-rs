use std::time::Duration;

use tokio::sync::mpsc;

pub struct LedDispatch {
  machine_to_dispatch: mpsc::UnboundedSender<MachineToLedDispatch>,
  dispatch_to_machine_receiver: mpsc::UnboundedReceiver<LedDispatchToMachine>,
  enabled: bool,
}

pub enum LedDispatchToMachine {
  UpdateSingle { led_id: u32, state: bool },
  UpdateMultiple { led_states: Vec<(u32, bool)> },
}

pub enum MachineToLedDispatch {
  Enable,
  Disable,
}
