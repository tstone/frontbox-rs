use std::sync::LazyLock;

use indexmap::IndexSet;

use crate::prelude::*;

impl From<Vec<DriverQuery>> for DriverQuery {
  fn from(value: Vec<DriverQuery>) -> Self {
    DriverQuery::any(value.iter().collect())
  }
}

impl From<Vec<SwitchQuery>> for SwitchQuery {
  fn from(value: Vec<SwitchQuery>) -> Self {
    SwitchQuery::any(value.iter().collect())
  }
}

impl From<Vec<LedQuery>> for LedQuery {
  fn from(value: Vec<LedQuery>) -> Self {
    LedQuery::any(value.iter().collect())
  }
}

impl From<LazyLock<SwitchDefinition>> for SwitchQuery {
  fn from(value: LazyLock<SwitchDefinition>) -> Self {
    SwitchQuery::name(value.name)
  }
}

impl From<&LazyLock<SwitchDefinition>> for SwitchQuery {
  fn from(value: &LazyLock<SwitchDefinition>) -> Self {
    SwitchQuery::name(value.name)
  }
}

impl From<LazyLock<DriverDefinition>> for DriverQuery {
  fn from(value: LazyLock<DriverDefinition>) -> Self {
    DriverQuery::name(value.name)
  }
}

impl From<&LazyLock<DriverDefinition>> for DriverQuery {
  fn from(value: &LazyLock<DriverDefinition>) -> Self {
    DriverQuery::name(value.name)
  }
}

impl From<LazyLock<LedDefinition>> for LedQuery {
  fn from(value: LazyLock<LedDefinition>) -> Self {
    LedQuery::names(value.names())
  }
}

impl From<&LazyLock<LedDefinition>> for LedQuery {
  fn from(value: &LazyLock<LedDefinition>) -> Self {
    LedQuery::names(value.names())
  }
}

impl From<Vec<&LazyLock<SwitchDefinition>>> for SwitchQuery {
  fn from(value: Vec<&LazyLock<SwitchDefinition>>) -> Self {
    let names: IndexSet<&'static str> = value.iter().map(|sw| sw.name).collect();
    SwitchQuery::names(names)
  }
}

impl From<Vec<&LazyLock<DriverDefinition>>> for DriverQuery {
  fn from(value: Vec<&LazyLock<DriverDefinition>>) -> Self {
    let names: IndexSet<&'static str> = value.iter().map(|dr| dr.name).collect();
    DriverQuery::names(names)
  }
}

impl From<Vec<&LazyLock<LedDefinition>>> for LedQuery {
  fn from(value: Vec<&LazyLock<LedDefinition>>) -> Self {
    // defs.names() yields a collection of String references; clone each String
    let names: IndexSet<String> = value
      .iter()
      .flat_map(|defs| defs.names().iter().cloned())
      .collect();
    LedQuery::names(&names)
  }
}
