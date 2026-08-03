#[derive(Debug, Clone, Copy)]
pub struct SystemHandle {
  pub id: u64,
  pub parent_key: &'static str,
}

impl SystemHandle {
  pub fn new(id: u64, parent_key: &'static str) -> Self {
    Self { id, parent_key }
  }
}
