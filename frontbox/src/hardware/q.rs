use std::any::TypeId;

use crate::prelude::*;

pub struct Q;

impl Q {
  /// Creates a selection that matches any switch/driver with the specified name.
  pub fn name(name: &'static str) -> HardwareQuery {
    HardwareQuery::Name(name)
  }

  /// Creates a selection that matches any of the provided names.
  pub fn names(names: Vec<&'static str>) -> HardwareQuery {
    names
      .into_iter()
      .map(HardwareQuery::Name)
      .reduce(Self::or)
      .unwrap()
  }

  /// Creates a selection that matches any switch/driver with the specified tag type.
  pub fn tag<T: Tag + 'static>() -> HardwareQuery {
    HardwareQuery::Tag(TypeId::of::<T>())
  }

  /// Creates a selection that matches if both sub-selections match.
  pub fn and(left: HardwareQuery, right: HardwareQuery) -> HardwareQuery {
    HardwareQuery::And(Box::new(left), Box::new(right))
  }

  /// Creates a selection that matches if either sub-selection matches.
  pub fn or(left: HardwareQuery, right: HardwareQuery) -> HardwareQuery {
    HardwareQuery::Or(Box::new(left), Box::new(right))
  }

  /// Sums up multiple selections with OR logic. Panics if the input is empty.
  pub fn any_of(selections: Vec<HardwareQuery>) -> HardwareQuery {
    selections.into_iter().reduce(Self::or).unwrap()
  }

  /// Sums up multiple selections with AND logic. Panics if the input is empty.
  pub fn all_of(selections: Vec<HardwareQuery>) -> HardwareQuery {
    selections.into_iter().reduce(Self::and).unwrap()
  }
}
