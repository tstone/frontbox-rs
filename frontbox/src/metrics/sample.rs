use std::marker::PhantomData;
use std::time::Duration;

use crate::metrics::{MetricDefinition, MetricKind};

/// A value measured once per occurrence, such as a game's duration or a player's score. See [metrics](crate::metrics).
#[derive(Clone, Debug)]
pub struct Sample<T> {
  definition: MetricDefinition,
  _type: PhantomData<fn() -> T>,
}

impl<T: SampleType> Sample<T> {
  pub fn new(label: &'static str, desc: &'static str) -> Self {
    Self {
      definition: MetricDefinition::new(
        label,
        desc,
        MetricKind::Sample {
          type_name: std::any::type_name::<T>(),
        },
        T::format,
      ),
      _type: PhantomData,
    }
  }

  /// Set the sample's `key`. Key must be unique but is derived from label by default. This force-sets it to a specific value for uniqueness.
  pub fn key(mut self, key: &'static str) -> Self {
    self.definition.key = key;
    self
  }

  /// Group the sample for display and for resets
  pub fn category(mut self, category: &'static str) -> Self {
    self.definition.category = Some(category);
    self
  }

  /// The untyped declaration, for listing alongside metrics of other kinds
  pub fn definition(&self) -> &MetricDefinition {
    &self.definition
  }
}

/// A Rust type that can be recorded as a [`Sample`], by mapping itself to the `f64` it's stored as
pub trait SampleType {
  fn to_sample(&self) -> f64;
  /// How a stored value is shown to the operator
  fn format(value: f64) -> String;
}

macro_rules! numeric_sample_type {
  ($($t:ty),*) => {
    $(
      impl SampleType for $t {
        fn to_sample(&self) -> f64 {
          *self as f64
        }

        fn format(value: f64) -> String {
          value.to_string()
        }
      }
    )*
  };
}

numeric_sample_type!(u8, u16, u32, u64, usize, i8, i16, i32, i64, f32, f64);

/// Stored as milliseconds
impl SampleType for Duration {
  fn to_sample(&self) -> f64 {
    self.as_secs_f64() * 1000.0
  }

  fn format(value: f64) -> String {
    let millis = Duration::from_millis(value.max(0.0) as u64);
    humantime::format_duration(millis).to_string()
  }
}
