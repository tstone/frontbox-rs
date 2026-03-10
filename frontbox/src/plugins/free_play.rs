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

impl CloneableSystem for FreePlay {
  fn on_event(&mut self, event: &dyn FrontboxEvent, _ctx: &Context, cmds: &mut Commands) {
    handle_event!(event, {
      SwitchClosed => |e| {
        if e.switch.name == self.start_button_id {
          cmds.start_game();
        }
      }
    });
  }
}
