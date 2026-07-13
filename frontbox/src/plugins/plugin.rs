use crate::prelude::App;

pub trait Plugin {
  fn build(&self, app: &mut App);
}

impl<T: Plugin + Clone> Plugin for &T {
  fn build(&self, app: &mut App) {
    (*self).clone().build(app)
  }
}
