use std::sync::LazyLock;

use indexmap::IndexSet;

use crate::prelude::*;

impl From<Vec<HardwareQuery>> for HardwareQuery {
  fn from(value: Vec<HardwareQuery>) -> Self {
    Q::any_of(value)
  }
}

impl From<LazyLock<SwitchDefinition>> for HardwareQuery {
  fn from(value: LazyLock<SwitchDefinition>) -> Self {
    Q::name(value.name)
  }
}

impl From<&LazyLock<SwitchDefinition>> for HardwareQuery {
  fn from(value: &LazyLock<SwitchDefinition>) -> Self {
    Q::name(value.name)
  }
}

impl From<LazyLock<DriverDefinition>> for HardwareQuery {
  fn from(value: LazyLock<DriverDefinition>) -> Self {
    Q::name(value.name)
  }
}

impl From<&LazyLock<DriverDefinition>> for HardwareQuery {
  fn from(value: &LazyLock<DriverDefinition>) -> Self {
    Q::name(value.name)
  }
}

impl From<LazyLock<LedDefinition>> for HardwareQuery {
  fn from(value: LazyLock<LedDefinition>) -> Self {
    Q::names(value.names())
  }
}

impl From<&LazyLock<LedDefinition>> for HardwareQuery {
  fn from(value: &LazyLock<LedDefinition>) -> Self {
    Q::names(value.names())
  }
}

impl From<Vec<&LazyLock<SwitchDefinition>>> for HardwareQuery {
  fn from(value: Vec<&LazyLock<SwitchDefinition>>) -> Self {
    let names: IndexSet<String> = value.iter().map(|sw| sw.name.to_string()).collect();
    Q::names(&names)
  }
}

impl From<Vec<&LazyLock<DriverDefinition>>> for HardwareQuery {
  fn from(value: Vec<&LazyLock<DriverDefinition>>) -> Self {
    let names: IndexSet<String> = value.iter().map(|dr| dr.name.to_string()).collect();
    Q::names(&names)
  }
}

impl From<Vec<&LazyLock<LedDefinition>>> for HardwareQuery {
  fn from(value: Vec<&LazyLock<LedDefinition>>) -> Self {
    // defs.names() yields a collection of String references; clone each String
    let names: IndexSet<String> = value
      .iter()
      .flat_map(|defs| defs.names().iter().cloned())
      .collect();
    Q::names(&names)
  }
}
