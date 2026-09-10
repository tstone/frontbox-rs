use crate::prelude::*;

/// A simple system that emits a QuitGame event whenever the left flipper button + start button is held down for over a second
pub struct QuitGameSystem {
  left_flipper_name: &'static str,
  left_flipper_down: bool,
  start_btn_name: &'static str,
  start_down: bool,
  cue_id: Option<u64>
}

impl QuitGameSystem {
  pub fn new(left_flipper_switch_name: &'static str, start_btn_name: &'static str) -> Self {
    Self {
      left_flipper_name: left_flipper_switch_name,
      left_flipper_down: false,
      start_btn_name,
      start_down: false,
      cue_id: None
    }  
  }

  fn on_flipper_down(&mut self, ctx: &SystemContext) {
    if self.left_flipper_down { return; }
    self.left_flipper_down = true;
    self.try_start_hold(ctx);
  }

  fn on_flipper_up(&mut self, ctx: &SystemContext) {
    self.left_flipper_down = false;
    self.clear_cue(ctx);
  }

  fn on_start_down(&mut self, ctx: &SystemContext) {
    if self.start_down { return; }
    self.start_down = true;
    self.try_start_hold(ctx);
  }

  fn on_start_up(&mut self, ctx: &SystemContext) {
    self.start_down = false;
    self.clear_cue(ctx);
  }

  fn clear_cue(&mut self, ctx: &SystemContext) {
    if let Some(cue_id) = self.cue_id {
      ctx.cancel_cue(cue_id);
      self.cue_id = None;
    }
  }

  fn try_start_hold(&mut self, ctx: &SystemContext) {
    if self.left_flipper_down && self.start_down {
      if self.cue_id.is_none() {
        self.cue_id = Some(ctx.cue(Held, Duration::from_millis(1250).once()));
      }
    }
  }

  fn on_held(&mut self, ctx: &SystemContext) {
    if self.left_flipper_down && self.start_down {
      ctx.emit(QuitGame);
    }
  }
}

impl System for QuitGameSystem {
  fn on_event(&mut self, event: &dyn Event, ctx: &SystemContext) {
    if event.is::<Held>() {
      self.on_held(ctx);
    } else if let Some(event) = event.downcast_ref::<SwitchClosed>() {
      if event.switch.name == self.left_flipper_name {
        self.on_flipper_down(ctx);
      } else if event.switch.name == self.start_btn_name {
        self.on_start_down(ctx);
      }
    } else if let Some(event) = event.downcast_ref::<SwitchOpened>() {
      if event.switch.name == self.left_flipper_name {
        self.on_flipper_up(ctx);
      } else if event.switch.name == self.start_btn_name {
        self.on_start_up(ctx);
      }
    }   
  }
}

#[derive(serde::Serialize, Event)]
struct Held;

#[derive(serde::Serialize, Event)]
pub struct QuitGame;