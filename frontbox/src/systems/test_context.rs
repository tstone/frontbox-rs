use std::collections::HashMap;

use tokio::sync::mpsc;

use crate::prelude::app_message::AppMessage;
use crate::prelude::*;

pub struct TestContext {
  pub base: BootSnapshot,
  pub groups: Groups,
  tx: mpsc::UnboundedSender<AppMessage>,
}

impl TestContext {
  pub fn svc_ctx(&self) -> ServiceContext<'_> {
    ServiceContext::new(&self.base, &self.groups, self.tx.clone())
  }

  pub fn sys_ctx(&self) -> SystemContext<'_> {
    SystemContext::new(
      &self.base,
      SystemHandle::default(),
      &self.groups,
      self.tx.clone(),
    )
  }
}

impl Default for TestContext {
  fn default() -> Self {
    let (tx, _) = mpsc::unbounded_channel();
    Self {
      base: BootSnapshot::default(),
      groups: HashMap::new(),
      tx,
    }
  }
}
