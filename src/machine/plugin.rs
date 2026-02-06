use crate::prelude::Machine;

pub trait Plugin {
  fn register(&self, machine: &mut Machine);
}
