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
  fn on_event(&mut self, event: &dyn FrontboxEvent, ctx: &mut Context) {
    handle_event!(event, {
      SwitchClosed => |e| {
        if e.switch.name == self.start_button_id {
          ctx.start_game();
        }
      }
    });
  }
}
