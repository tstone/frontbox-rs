use std::any::TypeId;

use indexmap::IndexSet;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub enum LedQuery {
  Every,
  Name(String),
  Names(IndexSet<String>),
  Tag(TypeId),
  And(Vec<LedQuery>),
  Or(Vec<LedQuery>),
  Location(ReferencePlane, Region),
  Skip(Box<LedQuery>, usize),
  Take(Box<LedQuery>, usize),
  Reverse(Box<LedQuery>),
}

impl LedQuery {
  /// Literally every LED
  pub fn every() -> Self {
    Self::Every
  }

  pub fn range(self, range: std::ops::Range<usize>) -> Self {
    self.skip(range.start).take(range.end)
  }

  pub fn skip(self, n: usize) -> Self {
    Self::Skip(Box::new(self), n)
  }

  pub fn take(self, n: usize) -> Self {
    Self::Take(Box::new(self), n)
  }

  pub fn reverse(self) -> Self {
    Self::Reverse(Box::new(self))
  }

  /// Query that matches any driver with the specified name.
  pub fn name(n: impl Into<String>) -> Self {
    Self::Name(n.into())
  }

  /// Query that matches any of the provided names.
  pub fn names<S: Into<String>>(names: impl IntoIterator<Item = S>) -> Self {
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

  /// Any of the given queries are sufficient ("join" operation)
  pub fn any(qs: Vec<&Self>) -> Self {
    Self::Or(qs.into_iter().map(|q| q.clone()).collect())
  }

  /// All of the given queries must be satisfied
  pub fn all(qs: Vec<&Self>) -> Self {
    Self::And(qs.into_iter().map(|q| q.clone()).collect())
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

  /// As queries get more complex, it can sometimes be useful to pre-compute them into a list of names rather than dynamically re-computing them each time they are needed.
  pub fn precompute(&self, ctx: &ServiceContext) -> Self {
    LedQuery::Names(self.query_iter(ctx).map(|led| led.name.clone()).collect())
  }

  /// Resolve the query into a reference for all matching LEDs
  pub fn query_iter<'c>(
    &'c self,
    ctx: &'c ServiceContext,
  ) -> Box<dyn Iterator<Item = &'c LED> + 'c> {
    ctx.leds.query_iter(&self)
  }

  /// Resolve the query into a reference for all matching LEDs
  pub fn query<'c>(&'c self, ctx: &'c ServiceContext) -> Vec<&'c LED> {
    ctx.leds.query_iter(&self).collect()
  }

  pub fn query_addresses<'c>(&'c self, ctx: &'c ServiceContext) -> Vec<LedAddress> {
    ctx
      .leds
      .query_iter(&self)
      .map(|led| led.address.clone())
      .collect()
  }
}
