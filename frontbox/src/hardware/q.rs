use std::any::TypeId;

use crate::prelude::*;

/// # Hardware Queries (`Q`)
///
/// Hardware queries are a way to describe what hardware to use.
///
/// ```rust
/// // select by name
/// let q = Q::name("foo");
/// let q = Q::names(vec!["foo", "bar"]);
///
/// // names can be referenced from definitions
/// let q = Q::name(left_inlane_led.name);
///
/// // But defintions also have a build-in query helper that's shorter
/// let q = left_inlane_led.q();
///
/// // Entire LED strips can be referenced by name
/// let q = Q::name(left_cabinet_strip.name);
/// // or specific children can be referenced
/// let q = Q::name(left_cabinet_strip.child(0).name);
/// // children have their own query helper too
/// let q = left_cabinet_strip.child(0).q();
///
/// // select by tag (see hardware definition below for more on tagging)
/// let q = Q::tag::<Playfield>();
///
/// // select things in a location
/// let q = Q::location(Location::Radius(x, y, r));
///
/// // multiple criteria
/// let q = Q::name("start_button").or(Q::tag::<StartButton>());
/// let q = Q::location((Location::Radius(x, y, r)))
///   .and(Q::tag::<Custom>()); // must be within location and have tag to match
///
/// // masking/exclusions
/// // select everything that is not tagged Playfield
/// let q = Q::not(Q::tag::<Playfield>);
/// // select everything tagged Playfield that is not also tagged Target
/// let q = Q::tag::<Playfield>().not(Q::tag::<Target>());
///
/// // other
/// let q = Q::all(); // select everything of this type
/// let q = Q::rand(10); // take 10 random, even if there are more
/// let q = Q::tag::<Playfield>().rand(10);
/// ```
///
/// Queries are just a description of hardware and don't contain the reference to matching hardware. However they can be used a predicate with an event or given `Context` to resolve into the matching hardware.
///
/// #### Query as Predicate
///
/// ```rust
/// // Get all matching switches (resolve query to hardware reference)
/// let switches = ctx.switches.query(q);
///
/// // ...
///
/// fn on_event(&mut self, event: &dyn Event, ctx: &Context) {
///   if let Some(e) = event.downcast_ref::<SwitchClosed>() {
///     // use as a predicate
///     if q.matches_switch(e.switch) {
///       // ...
///     }
///   }
/// }
/// ```
#[derive(Debug)]
pub struct Q;

impl Q {
  /// Query that matches any switch/driver with the specified name.
  pub fn name(name: &'static str) -> HardwareQuery {
    HardwareQuery::Name(name.to_string())
  }

  /// Query that matches any of the provided names.
  pub fn names<S: Into<String>>(names: impl IntoIterator<Item = S>) -> HardwareQuery {
    HardwareQuery::Names(names.into_iter().map(Into::into).collect())
  }

  /// Query that matches any switch/driver with the specified tag type.
  pub fn tag<'a, T: Tag + 'static>() -> HardwareQuery {
    HardwareQuery::Tag(TypeId::of::<T>())
  }

  /// Query that matches if both sub-selections match.
  pub fn and<'a>(left: HardwareQuery, right: HardwareQuery) -> HardwareQuery {
    left.and(right)
  }

  /// Query that matches if either sub-selection matches.
  pub fn or<'a>(left: HardwareQuery, right: HardwareQuery) -> HardwareQuery {
    left.or(right)
  }

  /// Any of the given queries are sufficient ("join" operation)
  pub fn any(qs: Vec<&HardwareQuery>) -> HardwareQuery {
    HardwareQuery::Or(qs.into_iter().map(|q| q.clone()).collect())
  }

  /// All of the given queries must be satisfied
  pub fn all(qs: Vec<&HardwareQuery>) -> HardwareQuery {
    HardwareQuery::And(qs.into_iter().map(|q| q.clone()).collect())
  }

  /// All matching within the given rectangle, on the given plane
  pub fn within_rect(plane: &ReferencePlane, top_left: Vec2, bottom_right: Vec2) -> HardwareQuery {
    HardwareQuery::Location(plane.clone(), Region::Rect { top_left, bottom_right })
  }

  /// All matching within the given circle, on the given plane
  pub fn within_radius(plane: &ReferencePlane, center: Vec2, radius: f32) -> HardwareQuery {
    HardwareQuery::Location(plane.clone(), Region::Circle { center, radius })
  }
}
