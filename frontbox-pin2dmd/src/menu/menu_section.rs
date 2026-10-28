use frontbox::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

static ROW_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct MenuSection {
  pub id: u64,
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
      id: MenuRow::next_row_id(),
      name: name.clone(),
      rows: vec![MenuRow::Heading(MenuRow::next_row_id(), name)],
    }
  }

  pub fn section(mut self, section: MenuSection) -> Self {
    self.rows.push(MenuRow::Section(section));
    self
  }

  pub fn configs(mut self, configs: Vec<&'static ConfigDefinition>) -> Self {
    for config in configs {
      let id = MenuRow::next_row_id();
      self.rows.push(MenuRow::Config(id, config));
    }
    self
  }

  pub fn header(mut self, text: impl Into<String>) -> Self {
    let id = MenuRow::next_row_id();
    self.rows.push(MenuRow::Heading(id, text.into()));
    self
  }

  pub fn action(
    mut self,
    text: &'static str,
    action: impl Fn(&SystemContext) + Send + Sync + 'static,
  ) -> Self {
    let id = MenuRow::next_row_id();
    self.rows.push(MenuRow::Action(id, text, Arc::new(action)));
    self
  }

  /// Add menu item to terminate just the software (useful for development)
  pub fn terminate(self) -> Self {
    self.action("Terminate", |ctx| {
      ctx.shutdown(ShutdownScope::Process);
    })
  }

  /// Add menu item to shutdown the whole computer
  pub fn shutdown(self) -> Self {
    self.action("Shutdown", |ctx| {
      ctx.shutdown(ShutdownScope::OperatingSystem);
    })
  }
}

#[derive(Clone)]
pub enum MenuRow {
  Section(MenuSection),
  Config(u64, &'static ConfigDefinition),
  Heading(u64, String),
  Special(u64, &'static str),
  Action(
    u64,
    &'static str,
    Arc<dyn Fn(&SystemContext) + Send + Sync + 'static>,
  ),
}

impl MenuRow {
  pub(crate) fn next_row_id() -> u64 {
    ROW_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
  }

  pub fn id(&self) -> u64 {
    match self {
      Self::Section(s) => s.id,
      Self::Config(id, _) => *id,
      Self::Heading(id, _) => *id,
      Self::Special(id, _) => *id,
      Self::Action(id, ..) => *id,
    }
  }

  pub fn section(&self) -> Option<&MenuSection> {
    match self {
      Self::Section(s) => Some(s),
      _ => None,
    }
  }

  pub fn config(&self) -> Option<&'static ConfigDefinition> {
    match self {
      Self::Config(_, cfg) => Some(cfg),
      _ => None,
    }
  }

  pub fn increment(&self, ctx: &SystemContext) -> String {
    match self {
      Self::Config(_, config) => config.format(ctx.config.inc(config)),
      _ => "".to_string(),
    }
  }

  pub fn decrement(&self, ctx: &SystemContext) -> String {
    match self {
      Self::Config(_, config) => config.format(ctx.config.dec(config)),
      _ => "".to_string(),
    }
  }
}
