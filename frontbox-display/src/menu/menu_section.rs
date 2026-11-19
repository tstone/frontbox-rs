use frontbox_core::prelude::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct MenuSection {
  pub name: String,
  pub rows: Vec<MenuRow>,
}

impl MenuSection {
  pub fn root() -> Self {
    Self::new("Frontbox")
  }

  pub fn new(name: impl Into<String>) -> Self {
    let name = name.into();
    MenuSection {
      name: name.clone(),
      rows: vec![MenuRow::Heading(name)],
    }
  }

  pub fn section(mut self, section: MenuSection) -> Self {
    self.rows.push(MenuRow::Section(section));
    self
  }

  pub fn configs(mut self, configs: Vec<&'static ConfigDefinition>) -> Self {
    self.rows.extend(configs.into_iter().map(MenuRow::Config));
    self
  }

  pub fn header(mut self, text: impl Into<String>) -> Self {
    self.rows.push(MenuRow::Heading(text.into()));
    self
  }

  pub fn action(
    mut self,
    text: &'static str,
    action: impl Fn(&SystemContext) + Send + Sync + 'static,
  ) -> Self {
    self
      .rows
      .push(MenuRow::Action(text, false, Arc::new(action)));
    self
  }

  /// Like [`action`](Self::action), but selecting it first asks the operator to confirm: select again runs it, back
  /// cancels
  pub fn action_with_confirm(
    mut self,
    text: &'static str,
    action: impl Fn(&SystemContext) + Send + Sync + 'static,
  ) -> Self {
    self
      .rows
      .push(MenuRow::Action(text, true, Arc::new(action)));
    self
  }

  /// Add menu item to terminate just the software (useful for development)
  pub fn terminate(self) -> Self {
    self.action_with_confirm("Terminate", |ctx| {
      ctx.shutdown(ShutdownScope::Process);
    })
  }

  /// Add menu item to shutdown the whole computer
  pub fn shutdown(self) -> Self {
    self.action_with_confirm("Shutdown", |ctx| {
      ctx.shutdown(ShutdownScope::OperatingSystem);
    })
  }
}

#[derive(Clone)]
pub enum MenuRow {
  Section(MenuSection),
  Config(&'static ConfigDefinition),
  Heading(String),
  Special(&'static str),
  /// The text, whether it asks to confirm before running, and what it runs
  Action(
    &'static str,
    bool,
    Arc<dyn Fn(&SystemContext) + Send + Sync + 'static>,
  ),
}

impl MenuRow {
  pub fn section(&self) -> Option<&MenuSection> {
    match self {
      Self::Section(s) => Some(s),
      _ => None,
    }
  }

  pub fn config(&self) -> Option<&'static ConfigDefinition> {
    match self {
      Self::Config(cfg) => Some(cfg),
      _ => None,
    }
  }
}
