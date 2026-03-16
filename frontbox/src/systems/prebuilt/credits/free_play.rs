use crate::prebuilt::ConsumeCredit;
use crate::prelude::*;

#[derive(Clone)]
pub struct FreePlay {
  start_button_id: &'static str,
}

impl FreePlay {
  pub fn new(start_button_id: &'static str) -> Box<Self> {
    Box::new(Self { start_button_id })
  }
}

impl System for FreePlay {
  fn on_event(&mut self, event: &dyn Event, ctx: &mut Context) {
    if let Some(e) = event.downcast::<SwitchClosed>() {
      if e.switch.name == self.start_button_id {
        ctx.emit(CreditedStart);
      }
    }
  }

  fn on_startup(&mut self, ctx: &mut Context) {
    ctx.register_command::<ConsumeCredit>(|_, _, _| {
      // no-op
    });
  }
}

/// When the start button is pressed and the game should start with a credit
pub struct CreditedStart;
