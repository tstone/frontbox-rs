use crate::metrics::{MetricDefinition, MetricKind};

/// Counts how many times something happened. See [metrics](crate::metrics).
#[derive(Clone, Debug)]
pub struct Counter {
  definition: MetricDefinition,
}

impl Counter {
  pub fn new(label: &'static str, desc: &'static str) -> Self {
    Self {
      definition: MetricDefinition::new(label, desc, MetricKind::Counter, format_count),
    }
  }

  /// Set the counter's `key`. Key must be unique but is derived from label by default. This force-sets it to a specific value for uniqueness.
  pub fn key(mut self, key: &'static str) -> Self {
    self.definition.key = key;
    self
  }

  /// Group the counter for display and for resets
  pub fn category(mut self, category: &'static str) -> Self {
    self.definition.category = Some(category);
    self
  }

  /// The untyped declaration, for listing alongside metrics of other kinds
  pub fn definition(&self) -> &MetricDefinition {
    &self.definition
  }
}

fn format_count(value: f64) -> String {
  (value as u64).to_string()
}
