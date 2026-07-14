use crate::prelude::*;

pub trait MachineExt {
  fn configure_driver(&self, driver: &'static str, mode: impl DriverMode + 'static);
  fn activate_driver(&self, driver: &'static str, mode: ActivationMode);
  fn deactivate_driver(&self, driver: &'static str, mode: DeactivationMode);
  fn configure_switch(
    &self,
    switch: &'static str,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
  );
}

impl<'a> MachineExt for Context<'a> {
  // TODO: allow DriverDefinition to be passed in directly
  fn configure_driver(&self, driver: &'static str, mode: impl DriverMode + 'static) {
    with_machine(self, |machine| {
      machine.configure_driver(driver, mode, self);
    });
  }

  fn activate_driver(&self, driver: &'static str, mode: ActivationMode) {
    with_machine(self, |machine| {
      machine.activate_driver(driver, mode, self);
    });
  }

  fn deactivate_driver(&self, driver: &'static str, mode: DeactivationMode) {
    with_machine(self, |machine| {
      machine.deactivate_driver(driver, mode, self);
    });
  }

  fn configure_switch(
    &self,
    switch: &'static str,
    inverted: bool,
    debounce_close: Option<Duration>,
    debounce_open: Option<Duration>,
  ) {
    with_machine(self, |machine| {
      machine.configure_switch(switch, inverted, debounce_close, debounce_open, self);
    });
  }
}

fn with_machine<T>(ctx: &Context, f: impl FnOnce(&mut Machine) -> T) {
  if let Some(mut machine) = ctx.systems.get::<Machine>() {
    f(&mut machine);
  } else {
    log::error!("Machine not running.");
  }
}
