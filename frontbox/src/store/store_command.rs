use crate::store::Store;

pub enum StoreCommand {
  Write(Box<dyn FnOnce(&mut Store) + Send>),
}
