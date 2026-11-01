#[derive(Clone, Debug)]
pub struct MetricDefinition {
  pub key: &'static str,
  /// What the operator sees
  pub label: &'static str,
  pub desc: &'static str,
  /// Groups metrics for display and for resets
  pub category: Option<&'static str>,
  pub kind: MetricKind,
  pub(crate) formatter: fn(f64) -> String,
}

impl MetricDefinition {
  pub(crate) fn new(
    label: &'static str,
    desc: &'static str,
    kind: MetricKind,
    formatter: fn(f64) -> String,
  ) -> Self {
    Self {
      key: label,
      label,
      desc,
      category: None,
      kind,
      formatter,
    }
  }

  /// How a value is shown to the operator
  pub fn format(&self, value: f64) -> String {
    (self.formatter)(value)
  }

  /// Whether values recorded through one definition mean the same through the other. Label, description and
  /// category may differ.
  pub fn is_compatible(&self, other: &MetricDefinition) -> bool {
    self.kind == other.kind
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricKind {
  Counter,
  /// The Rust type the sample is recorded as, so a `Duration` and a `u32` don't share a key
  Sample {
    type_name: &'static str,
  },
}
