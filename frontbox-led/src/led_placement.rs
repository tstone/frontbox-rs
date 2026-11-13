use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use frontbox_core::prelude::*;
use frontbox_gfx::layout::Line;
use frontbox_gfx::prelude::{Rgba, RgbaColor};
use frontbox_gfx::scene::{self, LayerFrame, Graph, Params, Sample, Visits};
use frontbox_scene::{Moment, Scene};

use crate::*;

/// Where a scene's LED layers land on the machine's LEDs, and what each LED shows from them. `LedSystem` keeps one for
/// each playback it draws, and the console one for the scene it previews, so both show the same.
#[derive(Debug, Clone)]
pub struct LedPlacement {
  layers: Vec<PlacedLayer>,
}

impl LedPlacement {
  /// Places every LED layer of `scene`. LEDs don't move, so this is only worked out once for a scene. A layer with no
  /// LEDs yet, or on a plane the machine doesn't have, reaches none.
  pub fn new(scene: &Scene, leds: &LedLookup, radius: f32) -> Self {
    let layers = scene.layers.iter().enumerate().filter_map(|(index, layer)| Some((index, layer.as_leds()?)));
    let layers = layers.map(|(index, layer)| {
      let surface = layer.surface.sampled_as();
      let plane = layer.surface.plane().and_then(|plane| leds.plane(plane));
      let placed = layer.leds.as_ref().map_or(Vec::new(), |query| place_leds(surface, leds, plane, query, radius));
      let (placed, graph) = match &layer.layer.graph {
        Some(_) => {
          let (placed, graph) = place_graph(placed, plane);
          (placed, Some(graph))
        }
        None => (placed, None),
      };
      let leds = placed.into_iter().map(|(led, at)| PlacedLed { address: led.address.clone(), name: led.name.clone(), at }).collect();
      PlacedLayer { index, z: layer.z, surface, plane, placed: leds, graph, visits: None }
    });
    Self { layers: layers.collect() }
  }

  /// The scene's LED layers, in its order
  pub fn layers(&self) -> &[PlacedLayer] {
    &self.layers
  }

  /// Works out the traversals of each layer with a graph for `moment`, unless they're kept from earlier in its cycle
  pub fn update_visits(&mut self, scene: &Scene, moment: &Moment) {
    for placed in &mut self.layers {
      placed.update_visits(scene, moment);
    }
  }

  /// The color of each LED a layer lands on, by z from the bottom. On one z the layers are drawn bottom to top in the
  /// scene's order. Layers with a graph use the visits from the last [`Self::update_visits`].
  pub fn colors(&self, scene: &Scene, moment: &Moment) -> Vec<(i8, &PlacedLed, Rgba<u8>)> {
    let mut by_z: BTreeMap<i8, HashMap<&LedAddress, (&PlacedLed, Rgba<u8>)>> = BTreeMap::new();
    for placed in &self.layers {
      let layer = scene.layers[placed.index].layer();
      let frame = placed.frame(scene, moment);
      let colors = by_z.entry(placed.z).or_default();
      for led in &placed.placed {
        let color = layer.sample(&led.at, &frame);
        let entry = colors.entry(&led.address).or_insert((led, Rgba([0, 0, 0, 0])));
        entry.1 = color.composite_over(entry.1);
      }
    }
    by_z.into_iter().flat_map(|(z, colors)| colors.into_values().map(move |(led, color)| (z, led, color))).collect()
  }
}

/// One LED layer of a scene, on the machine's LEDs
#[derive(Debug, Clone)]
pub struct PlacedLayer {
  /// Its place in the scene's layers
  pub index: usize,
  pub z: i8,
  pub surface: scene::Surface,
  /// The plane it covers. `None` on a `Sequence`, or on a plane the machine doesn't have.
  pub plane: Option<&'static ReferencePlane>,
  /// Its LEDs, each with where it samples the layer
  pub placed: Vec<PlacedLed>,
  /// For a layer with a graph, the one built from its LEDs
  pub graph: Option<Arc<Graph>>,
  /// The traversals of the cycle being played, kept until the cycle, playback seed, or params change
  visits: Option<(VisitsFor, Visits)>,
}

impl PlacedLayer {
  /// The size of what it's drawn on, which is its plane's
  pub fn size(&self) -> Vec2 {
    self.plane.map_or(Vec2::ZERO, |plane| plane.extent)
  }

  /// Its graph's traversals, as of the last [`LedPlacement::update_visits`]
  pub fn kept_visits(&self) -> Option<&Visits> {
    self.visits.as_ref().map(|(_, visits)| visits)
  }

  /// What to sample its layer with at `moment`
  pub fn frame<'a>(&'a self, scene: &'a Scene, moment: &Moment<'a>) -> LayerFrame<'a> {
    scene.frame(moment, self.surface, self.size(), self.kept_visits())
  }

  fn update_visits(&mut self, scene: &Scene, moment: &Moment) {
    let Some(graph) = &self.graph else {
      return;
    };
    let cycle = scene.timing.map_or(0, |timing| timing.cycle_index(moment.time));
    let current = VisitsFor { cycle, playback_seed: moment.playback_seed, params: moment.params.clone() };
    if self.visits.as_ref().is_none_or(|(kept, _)| *kept != current) {
      let frame = scene.frame(moment, self.surface, self.size(), None);
      self.visits = Some((current, scene.layers[self.index].layer().visits(graph, &frame)));
    }
  }
}

/// An LED a layer lands on
#[derive(Debug, Clone)]
pub struct PlacedLed {
  pub address: LedAddress,
  pub name: String,
  /// Where it samples the layer
  pub at: Sample,
}

/// The LEDs matching `query` that a layer on `surface` lands on, each with where it samples the layer.
///
/// On a `Sequence` they spread along it in order. An `Area` needs a `plane` to put them on, and without one reaches no
/// LEDs.
pub fn place_leds<'a>(
  surface: scene::Surface,
  leds: &'a LedLookup,
  plane: Option<&ReferencePlane>,
  query: &'a LedQ,
  radius: f32,
) -> Vec<(&'a LED, Sample)> {
  match (surface, plane) {
    (scene::Surface::Sequence, _) => {
      let matching: Vec<_> = leds.query_iter(query).collect();
      let line = Line::new(matching.len());
      matching.into_iter().zip(line.samples()).collect()
    }
    (scene::Surface::Area, Some(plane)) => place_on_plane(leds, plane, query, radius),
    (scene::Surface::Area, None) => Vec::new(),
  }
}

/// The LEDs matching `query`, each with where it samples a layer when projected straight onto `plane`, which may be past
/// the plane's extent. LEDs without a location don't take part.
fn place_on_plane<'a>(leds: &'a LedLookup, plane: &ReferencePlane, query: &'a LedQ, radius: f32) -> Vec<(&'a LED, Sample)> {
  leds.query_iter(query).filter_map(|led| Some((led, Sample::new(plane.to_relative(led.location?), radius)))).collect()
}

/// What a cycle's visits were worked out for
#[derive(Debug, Clone, PartialEq)]
struct VisitsFor {
  cycle: u32,
  playback_seed: u32,
  params: Params,
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;
  use frontbox_scene::{LedLayer, LedSurface};

  use super::*;

  static PLANE: ReferencePlane = ReferencePlane::new("Plane").extent(Vec2::new(10.0, 10.0)).build();

  /// LEDs "led0".. at these locations, with the plane added
  fn placed_leds(locations: &[Option<Vec3>]) -> TestContext {
    let mut context = TestContext::default();
    for (i, location) in locations.iter().enumerate() {
      let address = LedAddress::new(ExpAddress::default(), i as u8);
      context.insert_led(LED { name: format!("led{i}"), address, location: *location, ..Default::default() });
    }
    context.add_plane(&PLANE);
    context
  }

  fn at(x: f32, y: f32) -> Option<Vec3> {
    Some(Vec3::new(x, y, 0.0))
  }

  fn solid() -> Layer {
    Layer::new(Fill::new(FillNode::Solid(Color(Rgba::red()).into())))
  }

  fn graph_layer() -> Layer {
    let wave = Traversal::new("wave", NodeRef::name("led0"), TraversalAlgorithm::BreadthFirst { wander: 0.0.into() }, 1.0);
    solid().graph(GraphMask::new(vec![wave]))
  }

  #[test]
  fn place_leds_spreads_a_sequence_along_the_leds_in_order() {
    let context = placed_leds(&[None, None, None]);
    let query = LedQ::names(vec!["led2", "led0", "led1"]);
    let placed = place_leds(scene::Surface::Sequence, &context.base.leds, None, &query, DEFAULT_LED_RADIUS);
    let names: Vec<_> = placed.iter().map(|(led, _)| led.name.as_str()).collect();
    let spots: Vec<_> = placed.iter().map(|(_, sample)| sample.pos.x).collect();
    assert_eq!(names, ["led2", "led0", "led1"]);
    assert_eq!(spots, [0.0, 0.5, 1.0]);
  }

  #[test]
  fn place_leds_puts_an_area_on_the_plane_with_the_radius_its_given() {
    let context = placed_leds(&[at(2.0, 3.0), at(50.0, 3.0), None]);
    let query = LedQ::every();
    let placed = place_leds(scene::Surface::Area, &context.base.leds, Some(&PLANE), &query, 0.25);
    assert_eq!(placed.len(), 2);
    assert_eq!(placed[0].0.name, "led0");
    assert_eq!(placed[0].1, Sample::new(Vec2::new(2.0, 3.0), 0.25));
    assert_eq!(placed[1].1, Sample::new(Vec2::new(50.0, 3.0), 0.25));
  }

  #[test]
  fn place_leds_reaches_nothing_on_an_area_without_a_plane() {
    let context = placed_leds(&[at(2.0, 3.0)]);
    let query = LedQ::every();
    assert!(place_leds(scene::Surface::Area, &context.base.leds, None, &query, DEFAULT_LED_RADIUS).is_empty());
  }

  #[test]
  fn only_led_layers_are_placed_and_keep_their_place_in_the_scene() {
    let context = placed_leds(&[at(2.0, 3.0)]);
    let scene = Scene::new(vec![
      frontbox_scene::DisplayLayer::new("Dmd", solid()).into(),
      LedLayer::area(LedQ::every(), "Plane", solid()).at_z(2).into(),
    ]);
    let placement = LedPlacement::new(&scene, &context.base.leds, DEFAULT_LED_RADIUS);
    let placed = &placement.layers()[0];
    assert_eq!((placement.layers().len(), placed.index, placed.z), (1, 1, 2));
    assert_eq!(placed.size(), PLANE.extent);
    assert_eq!(placed.placed[0].name, "led0");
  }

  #[test]
  fn a_layer_on_a_missing_plane_or_with_no_leds_reaches_none() {
    let context = placed_leds(&[at(2.0, 3.0)]);
    let no_leds = LedLayer { leds: None, z: 0, surface: LedSurface::Sequence, layer: solid() };
    let scene = Scene::new(vec![LedLayer::area(LedQ::every(), "Nowhere", solid()).into(), no_leds.into()]);
    let placement = LedPlacement::new(&scene, &context.base.leds, DEFAULT_LED_RADIUS);
    assert!(placement.layers().iter().all(|placed| placed.placed.is_empty()));
    assert_eq!(placement.layers()[0].plane, None);
  }

  #[test]
  fn a_graph_layer_numbers_its_leds_as_nodes() {
    let context = placed_leds(&[at(1.0, 5.0), at(2.0, 5.0), at(3.0, 5.0)]);
    let scene = Scene::new(vec![LedLayer::area(LedQ::every(), "Plane", graph_layer()).into()]);
    let placement = LedPlacement::new(&scene, &context.base.leds, DEFAULT_LED_RADIUS);
    let placed = &placement.layers()[0];
    assert_eq!(placed.graph.as_ref().unwrap().nodes().len(), 3);
    assert_eq!(placed.placed.iter().map(|led| led.at.node).collect::<Vec<_>>(), [Some(0), Some(1), Some(2)]);
  }

  #[test]
  fn the_same_leds_share_a_graph() {
    let context = placed_leds(&[at(1.0, 5.0), at(2.0, 5.0), at(3.0, 5.0), at(4.0, 5.0)]);
    let graph = || {
      let every = LedQ::every();
      let placed = place_leds(scene::Surface::Area, &context.base.leds, Some(&PLANE), &every, DEFAULT_LED_RADIUS);
      place_graph(placed, Some(&PLANE)).1
    };
    assert!(Arc::ptr_eq(&graph(), &graph()));
    assert_eq!(graph().edges().count(), 3);
  }
}
