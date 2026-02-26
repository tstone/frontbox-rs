use crate::prelude::*;

pub type Scene = Vec<SystemContainer>;

/// A district manages which stack of scenes is currently active, acting as a switchboard operator
#[allow(unused)]
pub trait District {
  fn split(self: Box<Self>) -> (Box<dyn SystemDistrict>, Box<dyn StorageDistrict>);
}

#[allow(unused)]
pub trait SystemDistrict {
  fn get_current(&self) -> &Scene;
  fn get_current_mut(&mut self) -> &mut Scene;

  fn on_district_enter(&self, ctx: &mut Context) {}
  fn on_add_player(&mut self, player_index: u8) {}
  fn on_change_player(&mut self, player_index: u8) {}
  fn on_district_exit(&mut self, ctx: &mut Context) {}
}

#[allow(unused)]
pub trait StorageDistrict {
  fn get_current(&self) -> &Store;
  fn get_current_mut(&mut self) -> &mut Store;

  fn on_add_player(&mut self, player_index: u8) {}
  fn on_change_player(&mut self, player_index: u8) {}
}
