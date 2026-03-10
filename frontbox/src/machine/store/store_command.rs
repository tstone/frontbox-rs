use crate::prelude::Store;

pub enum StoreCommand {
  StoreWrite(Box<dyn FnOnce(&mut Store) + Send>),
}
