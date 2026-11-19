use std::sync::Arc;

use crate::prelude::*;

/// A section of the operator menu: a name, and rows that are settings, actions, screens, or further sections. It only
/// describes the menu. Each display that shows it (`DmdMenuSystem` in `frontbox-display`) decides how to lay it out,
/// how the operator moves through it, and how values are changed.
///
/// ```rust,ignore
/// MenuSection::root()
///   .section(MenuSection::new("Left Flipper").configs(LEFT_FLIPPER.configs()))
///   .section(MenuSection::new("Game").configs(vec![MAX_EXTRA_BALLS.display()]))
///   .about()
///   .shutdown()
/// ```
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
    MenuSection {
      name: name.into(),
      rows: Vec::new(),
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

  /// A heading for the rows added after it
  pub fn header(mut self, text: impl Into<String>) -> Self {
    self.rows.push(MenuRow::Heading(text.into()));
    self
  }

  pub fn action(
    mut self,
    text: &'static str,
    action: impl Fn(&ServiceContext<'_>) + Send + Sync + 'static,
  ) -> Self {
    self
      .rows
      .push(MenuRow::Action(text, false, Arc::new(action)));
    self
  }

  /// Like [`action`](Self::action), but selecting it first asks the operator to confirm
  pub fn action_with_confirm(
    mut self,
    text: &'static str,
    action: impl Fn(&ServiceContext<'_>) + Send + Sync + 'static,
  ) -> Self {
    self
      .rows
      .push(MenuRow::Action(text, true, Arc::new(action)));
    self
  }

  /// Add the screen showing Frontbox's version
  pub fn about(mut self) -> Self {
    self.rows.push(MenuRow::Screen(MenuScreen::About));
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
  /// A setting the operator can change
  Config(&'static ConfigDefinition),
  /// Groups the rows after it, and can't be selected
  Heading(String),
  Screen(MenuScreen),
  /// The text, whether it asks to confirm before running, and what it runs. It's given a `ServiceContext` so it doesn't
  /// act as whichever display's system ran it.
  Action(
    &'static str,
    bool,
    Arc<dyn Fn(&ServiceContext<'_>) + Send + Sync + 'static>,
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

/// A screen of its own that the display draws, rather than a list of rows
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuScreen {
  /// Frontbox's version
  About,
}

impl MenuScreen {
  /// The text of the row that opens it
  pub fn label(&self) -> &'static str {
    match self {
      Self::About => "About",
    }
  }
}
