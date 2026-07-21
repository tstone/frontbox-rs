use std::sync::Arc;

use frontbox::prelude::*;

#[derive(Clone)]
pub struct MenuSection {
  pub name: &'static str,
  pub sections: Vec<MenuSection>,
  pub configs: Vec<Arc<&'static dyn GeneralizedConfigValue>>,
}

impl MenuSection {
  pub fn root() -> Self {
    Self::new("ROOT")
  }

  pub fn new(name: &'static str) -> Self {
    MenuSection {
      name,
      sections: Vec::new(),
      configs: Vec::new(),
    }
  }

  pub fn section(mut self, section: MenuSection) -> Self {
    self.sections.push(section);
    self
  }

  pub fn configs(mut self, configs: Vec<&'static dyn GeneralizedConfigValue>) -> Self {
    for config in configs {
      self.configs.push(Arc::new(config));
    }
    self
  }

  pub fn child_count(&self) -> usize {
    self.sections.len() + self.configs.len()
  }
}
