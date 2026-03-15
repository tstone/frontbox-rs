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

impl ChildSystem for FreePlay {
  fn on_event(&mut self, event: &dyn Event, _ctx: &Context_OLD, cmds: &mut Commands) {
    handle_event!(event, {
      SwitchClosed => |e| {
        if e.switch.name == self.start_button_id {
          cmds.game.start();
        }
      }
    });
  }
}
