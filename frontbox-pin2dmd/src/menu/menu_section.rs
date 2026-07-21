use frontbox::prelude::*;

pub struct MenuSection {
  name: &'static str,
  sections: Vec<MenuSection>,
  configs: Vec<Box<&'static dyn GeneralizedConfigValue>>,
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
      self.configs.push(Box::new(config));
    }
    self
  }
}
