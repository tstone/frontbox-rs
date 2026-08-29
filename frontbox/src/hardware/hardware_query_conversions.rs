use std::sync::LazyLock;

use indexmap::IndexSet;

use crate::prelude::*;

impl From<Vec<DriverQ>> for DriverQ {
  fn from(value: Vec<DriverQ>) -> Self {
    DriverQ::any(value.iter().collect())
  }
}

impl From<Vec<SwitchQ>> for SwitchQ {
  fn from(value: Vec<SwitchQ>) -> Self {
    SwitchQ::any(value.iter().collect())
  }
}

impl From<Vec<LedQ>> for LedQ {
  fn from(value: Vec<LedQ>) -> Self {
    LedQ::any(value.iter().collect())
  }
}

impl From<LazyLock<SwitchDefinition>> for SwitchQ {
  fn from(value: LazyLock<SwitchDefinition>) -> Self {
    SwitchQ::name(value.name)
  }
}

impl From<&LazyLock<SwitchDefinition>> for SwitchQ {
  fn from(value: &LazyLock<SwitchDefinition>) -> Self {
    SwitchQ::name(value.name)
  }
}

impl From<LazyLock<DriverDefinition>> for DriverQ {
  fn from(value: LazyLock<DriverDefinition>) -> Self {
    DriverQ::name(value.name)
  }
}

impl From<&LazyLock<DriverDefinition>> for DriverQ {
  fn from(value: &LazyLock<DriverDefinition>) -> Self {
    DriverQ::name(value.name)
  }
}

impl From<LazyLock<LedDefinition>> for LedQ {
  fn from(value: LazyLock<LedDefinition>) -> Self {
    LedQ::names(value.names())
  }
}

impl From<&LazyLock<LedDefinition>> for LedQ {
  fn from(value: &LazyLock<LedDefinition>) -> Self {
    LedQ::names(value.names())
  }
}

impl From<Vec<&LazyLock<SwitchDefinition>>> for SwitchQ {
  fn from(value: Vec<&LazyLock<SwitchDefinition>>) -> Self {
    let names: IndexSet<&'static str> = value.iter().map(|sw| sw.name).collect();
    SwitchQ::names(names)
  }
}

impl From<Vec<&LazyLock<DriverDefinition>>> for DriverQ {
  fn from(value: Vec<&LazyLock<DriverDefinition>>) -> Self {
    let names: IndexSet<&'static str> = value.iter().map(|dr| dr.name).collect();
    DriverQ::names(names)
  }
}

impl From<Vec<&LazyLock<LedDefinition>>> for LedQ {
  fn from(value: Vec<&LazyLock<LedDefinition>>) -> Self {
    // defs.names() yields a collection of String references; clone each String
    let names: IndexSet<String> = value
      .iter()
      .flat_map(|defs| defs.names().iter().cloned())
      .collect();
    LedQ::names(&names)
  }
}
