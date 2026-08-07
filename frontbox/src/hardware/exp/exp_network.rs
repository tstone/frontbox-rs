use crate::hardware::ExpBoard;

#[derive(Default)]
pub struct ExpNetwork {
  pub boards: Vec<ExpBoard>
}

impl ExpNetwork {
  pub fn new(boards: Vec<ExpBoard>) -> Self {
    Self { boards }
  }

  pub fn empty() -> Self {
    Self::new(Vec::new())
  }
}
