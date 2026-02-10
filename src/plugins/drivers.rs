use crate::prelude::*;
pub struct TriggerDriver(pub &'static str);

impl Command for TriggerDriver {
  fn execute(&self, _system_id: usize, machine: &mut Machine) {
    machine.trigger_driver(self.0);
  }
}
