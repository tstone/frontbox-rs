use crate::prelude::*;

pub trait Plugin {
  fn register(&self, machine: &mut MachineBuilder);
}
