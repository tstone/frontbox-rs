use std::any::TypeId;

use indexmap::IndexSet;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum SwitchQ {
  Name(&'static str),
  Names(IndexSet<&'static str>),
  Tag(TypeId),
  And(Vec<SwitchQ>),
  Or(Vec<SwitchQ>),
  Location(ReferencePlane, Region),
}

impl SwitchQ {
  /// Query that matches any driver with the specified name.
  pub fn name(n: &'static str) -> Self {
    Self::Name(n)
  }

  /// Query that matches any of the provided names.
  pub fn names(names: impl IntoIterator<Item = &'static str>) -> Self {
    Self::Names(names.into_iter().map(Into::into).collect())
  }

  /// Query that matches any driver with the specified tag type.
  pub fn tag<'a, T: Tag + 'static>() -> Self {
    Self::Tag(TypeId::of::<T>())
  }

  /// Query that matches if either sub-selection matches.
  pub fn or(self, other: Self) -> Self {
    Self::Or(vec![self, other])
  }

  /// Any of the given queries are sufficient ("join" operation)
  pub fn and(self, other: Self) -> Self {
    Self::And(vec![self, other])
  }

  /// All matching within the given rectangle, on the given plane
  pub fn within_rect(plane: &ReferencePlane, top_left: Vec2, bottom_right: Vec2) -> Self {
    Self::Location(
      plane.clone(),
      Region::Rect {
        top_left,
        bottom_right,
      },
    )
  }

  /// All matching within the given circle, on the given plane
  pub fn within_radius(plane: &ReferencePlane, center: Vec2, radius: f32) -> Self {
    Self::Location(plane.clone(), Region::Circle { center, radius })
  }

  /// Any of the given queries are sufficient ("join" operation)
  pub fn any(qs: Vec<&Self>) -> Self {
    Self::Or(qs.into_iter().map(|q| q.clone()).collect())
  }

  /// All of the given queries must be satisfied
  pub fn all(qs: Vec<&Self>) -> Self {
    Self::And(qs.into_iter().map(|q| q.clone()).collect())
  }

  /// As queries get more complex, it can sometimes be useful to pre-compute them into a list of names rather than dynamically re-computing them each time they are needed.
  pub fn precompute(&self, ctx: &ServiceContext) -> Self {
    SwitchQ::Names(self.query_names(ctx).into_iter().map(Into::into).collect())
  }

  pub fn matches(&self, switch: &Switch) -> bool {
    match self {
      Self::Name(name) => switch.name == *name,
      Self::Names(names) => names.contains(switch.name),
      Self::Tag(tag) => switch.has_typed_tag(*tag),
      Self::And(qs) => qs.iter().all(|q| q.matches(switch)),
      Self::Or(qs) => qs.iter().any(|q| q.matches(switch)),
      Self::Location(plane, region) => switch
        .location
        .map(|location| region.within(plane.to_relative(location)))
        .unwrap_or(false),
    }
  }

  /// Resolve the query into a reference for all matching Switches
  pub fn query_iter<'a>(&self, ctx: &'a ServiceContext) -> impl Iterator<Item = &'a Switch> {
    ctx.switches.query_iter(&self)
  }

  /// Resolve the query into a reference for all matching Switches
  pub fn query<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'a Switch> {
    self.query_iter(ctx).collect()
  }

  /// Resolve the query into a the names of all matching Switches
  pub fn query_names<'a>(&self, ctx: &'a ServiceContext) -> Vec<&'static str> {
    self.query_iter(ctx).map(|sw| sw.name).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tags::Playfield;

  #[test]
  fn name_query() {
    let q = SwitchQ::name("switch1");

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![],
      location: None,
    };

    assert!(q.matches(&switch));
  }

  #[test]
  fn tag_query() {
    let q = SwitchQ::tag::<Playfield>();

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(q.matches(&switch));
  }

  #[test]
  fn and_query() {
    let q = SwitchQ::and(SwitchQ::name("switch1"), SwitchQ::tag::<Playfield>());

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(q.matches(&switch));
  }

  #[test]
  fn or_query() {
    let q = SwitchQ::or(SwitchQ::name("switch1"), SwitchQ::name("switch2"));

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(q.matches(&switch));
  }

  #[test]
  fn any_of_query() {
    let q = SwitchQ::any(vec![&SwitchQ::name("switch1"), &SwitchQ::name("switch2")]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(q.matches(&switch));
  }

  #[test]
  fn all_of_query() {
    let q = SwitchQ::all(vec![
      &SwitchQ::name("switch1"),
      &SwitchQ::tag::<Playfield>(),
    ]);

    let switch = Switch {
      id: 1,
      name: "switch1",
      assignment: IoAddress {
        board_idx: 0,
        pin: 1,
      },
      tags: vec![Box::new(Playfield)],
      location: None,
    };

    assert!(q.matches(&switch));
  }
}
