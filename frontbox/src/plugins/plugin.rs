use crate::prelude::App;

pub trait Plugin {
  fn register(&self, app: &mut App);
}
