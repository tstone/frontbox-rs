use tokio::sync::mpsc;

use crate::prelude::{Store, StoreCommand};

#[derive(Clone)]
pub struct WriteableStore {
  pub(crate) sender: mpsc::UnboundedSender<StoreCommand>,
}

impl WriteableStore {
  pub fn new(sender: mpsc::UnboundedSender<StoreCommand>) -> Self {
    Self { sender }
  }

  pub fn write(&self, f: impl FnOnce(&mut Store) + Send + 'static) {
    let _ = self.sender.send(StoreCommand::Write(Box::new(f)));
  }
}
