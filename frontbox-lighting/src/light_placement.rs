use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use frontbox_core::prelude::*;
use frontbox_gfx::layout::Line;
use frontbox_gfx::prelude::Rgba;
use frontbox_gfx::scene::{self, Graph, Layer, LayerFrame, Sample, Variables, Visits};
use frontbox_scene::{Moment, Scene};

use crate::*;

/// Where a scene's light layers land on the machine's lights, and what each light shows from them. `LayeredLightManager` keeps one for
/// each playback it draws, and the console one for the scene it previews, so both show the same.
#[derive(Debug, Clone)]
pub struct LightPlacement {
  layers: Vec<PlacedLayer>,
}

impl LightPlacement {
  /// Places every light layer of `scene`. Lights don't move, so this is only worked out once for a scene. A layer with no
  /// lights yet, or on a plane the machine doesn't have, reaches none.
  pub fn new(scene: &Scene, lights: &LightLookup, radius: f32) -> Self {
    let layers = scene
      .layers
      .iter()
      .enumerate()
      .filter_map(|(index, layer)| Some((index, layer.as_lights()?)));
    let layers = layers.map(|(index, layer)| {
      let surface = layer.surface.sampled_as();
      let plane = layer.surface.plane().and_then(|plane| lights.plane(plane));
      let placed = layer.lights.as_ref().map_or(Vec::new(), |query| {
        place_lights(surface, lights, plane, query, radius)
      });
      let (placed, graph) = match &layer.layer.graph {
        Some(_) => {
          let (placed, graph) = place_graph(placed, plane);
          (placed, Some(graph))
        }
        None => (placed, None),
      };
      let lights = placed
        .into_iter()
        .map(|(light, at)| PlacedLight {
          address: light.address.clone(),
          name: light.name.clone(),
          at,
        })
        .collect();
      PlacedLayer {
        index,
        z: layer.z,
        surface,
        plane,
        placed: lights,
        graph,
        visits: None,
      }
    });
    Self {
      layers: layers.collect(),
    }
  }

  /// The scene's light layers, in its order
  pub fn layers(&self) -> &[PlacedLayer] {
    &self.layers
  }

  /// Works out the traversals of each layer with a graph for `moment`, unless they're kept from earlier in its cycle
  pub fn update_visits(&mut self, scene: &Scene, moment: &Moment) {
    for placed in &mut self.layers {
      placed.update_visits(scene, moment);
    }
  }

  /// The color of each light a layer lands on, by z from the bottom. On one z the layers are drawn bottom to top in the
  /// scene's draw order, and a mask masks the layers below it on the lights it lands on, leaving other lights as they
  /// are.
  /// Layers with a graph use the visits from the last [`Self::update_visits`].
  pub fn colors(&self, scene: &Scene, moment: &Moment) -> Vec<(i8, &PlacedLight, Rgba<u8>)> {
    let mut by_z: BTreeMap<i8, HashMap<&LightAddress, (&PlacedLight, Rgba<u8>)>> = BTreeMap::new();
    for step in scene.draw_order() {
      let Some(placed) = self.layers.iter().find(|placed| placed.index == step.layer) else {
        continue;
      };
      let (Some(layer), Some(first)) = (placed.layer(scene), placed.placed.first()) else {
        continue;
      };
      let frame = placed.frame(scene, moment);
      let layer = layer.prepare(&frame, first.at.radius);
      let part = scene.drawn_part(step, &frame, first.at.radius);
      let z = scene.step_z(step).unwrap_or(placed.z);
      let colors = by_z.entry(z).or_default();
      for light in &placed.placed {
        if !part.covers(&light.at) {
          continue;
        }
        let entry = colors
          .entry(&light.address)
          .or_insert((light, Rgba([0, 0, 0, 0])));
        entry.1 = layer.composite(&light.at, entry.1);
      }
    }
    by_z
      .into_iter()
      .flat_map(|(z, colors)| {
        colors
          .into_values()
          .map(move |(light, color)| (z, light, color))
      })
      .collect()
  }
}

/// One light layer of a scene, on the machine's lights
#[derive(Debug, Clone)]
pub struct PlacedLayer {
  /// Its place in the scene's layers
  pub index: usize,
  pub z: i8,
  pub surface: scene::Surface,
  /// The plane it covers. `None` on a `Sequence`, or on a plane the machine doesn't have.
  pub plane: Option<&'static ReferencePlane>,
  /// Its lights, each with where it samples the layer
  pub placed: Vec<PlacedLight>,
  /// For a layer with a graph, the one built from its lights
  pub graph: Option<Arc<Graph>>,
  /// The traversals of the cycle being played, kept until the cycle, playback seed, or variables change
  visits: Option<(VisitsFor, Visits)>,
}

impl PlacedLayer {
  /// The size of what it's drawn on, which is its plane's
  pub fn size(&self) -> Vec2 {
    self.plane.map_or(Vec2::ZERO, |plane| plane.extent)
  }

  /// Its graph's traversals, as of the last [`LightPlacement::update_visits`]
  pub fn kept_visits(&self) -> Option<&Visits> {
    self.visits.as_ref().map(|(_, visits)| visits)
  }

  /// What it draws, from the scene it was placed from
  pub fn layer<'a>(&self, scene: &'a Scene) -> Option<&'a Layer> {
    scene
      .layers
      .get(self.index)?
      .as_lights()
      .map(|layer| &layer.layer)
  }

  /// What to sample its layer with at `moment`
  pub fn frame<'a>(&'a self, scene: &'a Scene, moment: &Moment<'a>) -> LayerFrame<'a> {
    scene.frame(moment, self.surface, self.size(), self.kept_visits())
  }

  fn update_visits(&mut self, scene: &Scene, moment: &Moment) {
    let Some(graph) = &self.graph else {
      return;
    };
    let cycle = scene
      .timing
      .map_or(0, |timing| timing.cycle_index(moment.time));
    let current = VisitsFor {
      cycle,
      playback_seed: moment.playback_seed,
      variables: moment.variables.clone(),
    };
    if self
      .visits
      .as_ref()
      .is_none_or(|(kept, _)| *kept != current)
    {
      let frame = scene.frame(moment, self.surface, self.size(), None);
      if let Some(layer) = self.layer(scene) {
        self.visits = Some((current, layer.visits(graph, &frame)));
      }
    }
  }
}

/// A light a layer lands on
#[derive(Debug, Clone)]
pub struct PlacedLight {
  pub address: LightAddress,
  pub name: String,
  /// Where it samples the layer
  pub at: Sample,
}

/// The lights matching `query` that a layer on `surface` lands on, each with where it samples the layer.
///
/// On a `Sequence` they spread along it in order. An `Area` needs a `plane` to put them on, and without one reaches no
/// lights.
pub fn place_lights<'a>(
  surface: scene::Surface,
  lights: &'a LightLookup,
  plane: Option<&ReferencePlane>,
  query: &'a LightQ,
  radius: f32,
) -> Vec<(&'a Light, Sample)> {
  match (surface, plane) {
    (scene::Surface::Sequence, _) => {
      let matching: Vec<_> = lights.query_iter(query).collect();
      let line = Line::new(matching.len());
      matching.into_iter().zip(line.samples()).collect()
    }
    (scene::Surface::Area, Some(plane)) => place_on_plane(lights, plane, query, radius),
    (scene::Surface::Area, None) => Vec::new(),
  }
}

/// The lights matching `query`, each with where it samples a layer when projected straight onto `plane`, which may be past
/// the plane's extent. Lights without a location don't take part.
fn place_on_plane<'a>(
  lights: &'a LightLookup,
  plane: &ReferencePlane,
  query: &'a LightQ,
  radius: f32,
) -> Vec<(&'a Light, Sample)> {
  lights
    .query_iter(query)
    .filter_map(|light| {
      Some((
        light,
        Sample::new(plane.to_relative(light.location?), radius),
      ))
    })
    .collect()
}

/// What a cycle's visits were worked out for
#[derive(Debug, Clone, PartialEq)]
struct VisitsFor {
  cycle: u32,
  playback_seed: u32,
  variables: Variables,
}

#[cfg(test)]
mod tests {
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;
  use frontbox_scene::{LightLayer, LightSurface};

  use super::*;

  static PLANE: ReferencePlane = ReferencePlane::new("Plane")
    .extent(Vec2::new(10.0, 10.0))
    .build();

  /// Lights "light0".. at these locations, with the plane added
  fn placed_lights(locations: &[Option<Vec3>]) -> TestContext {
    let mut context = TestContext::default();
    for (i, location) in locations.iter().enumerate() {
      let address = LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, i as u8));
      context.insert_light(Light {
        name: format!("light{i}"),
        address,
        location: *location,
        ..Default::default()
      });
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
    let wave = Traversal::new(
      "wave",
      NodeRef::name("light0"),
      TraversalAlgorithm::BreadthFirst { wander: 0.0.into() },
      1.0,
    );
    solid().graph(GraphMask::new(vec![wave]))
  }

  #[test]
  fn place_lights_spreads_a_sequence_along_the_lights_in_order() {
    let context = placed_lights(&[None, None, None]);
    let query = LightQ::names(vec!["light2", "light0", "light1"]);
    let placed = place_lights(
      scene::Surface::Sequence,
      &context.base.lights,
      None,
      &query,
      DEFAULT_LIGHT_RADIUS,
    );
    let names: Vec<_> = placed
      .iter()
      .map(|(light, _)| light.name.as_str())
      .collect();
    let spots: Vec<_> = placed.iter().map(|(_, sample)| sample.pos.x).collect();
    assert_eq!(names, ["light2", "light0", "light1"]);
    assert_eq!(spots, [0.0, 0.5, 1.0]);
  }

  #[test]
  fn place_lights_puts_an_area_on_the_plane_with_the_radius_its_given() {
    let context = placed_lights(&[at(2.0, 3.0), at(50.0, 3.0), None]);
    let query = LightQ::every();
    let placed = place_lights(
      scene::Surface::Area,
      &context.base.lights,
      Some(&PLANE),
      &query,
      0.25,
    );
    assert_eq!(placed.len(), 2);
    assert_eq!(placed[0].0.name, "light0");
    assert_eq!(placed[0].1, Sample::new(Vec2::new(2.0, 3.0), 0.25));
    assert_eq!(placed[1].1, Sample::new(Vec2::new(50.0, 3.0), 0.25));
  }

  #[test]
  fn place_lights_reaches_nothing_on_an_area_without_a_plane() {
    let context = placed_lights(&[at(2.0, 3.0)]);
    let query = LightQ::every();
    assert!(
      place_lights(
        scene::Surface::Area,
        &context.base.lights,
        None,
        &query,
        DEFAULT_LIGHT_RADIUS
      )
      .is_empty()
    );
  }

  #[test]
  fn only_light_layers_are_placed_and_keep_their_place_in_the_scene() {
    let context = placed_lights(&[at(2.0, 3.0)]);
    let scene = Scene::new(vec![
      frontbox_scene::DisplayLayer::new("Dmd", solid()).into(),
      LightLayer::area(LightQ::every(), "Plane", solid())
        .at_z(2)
        .into(),
    ]);
    let placement = LightPlacement::new(&scene, &context.base.lights, DEFAULT_LIGHT_RADIUS);
    let placed = &placement.layers()[0];
    assert_eq!(
      (placement.layers().len(), placed.index, placed.z),
      (1, 1, 2)
    );
    assert_eq!(placed.size(), PLANE.extent);
    assert_eq!(placed.placed[0].name, "light0");
  }

  #[test]
  fn a_layer_on_a_missing_plane_or_with_no_lights_reaches_none() {
    let context = placed_lights(&[at(2.0, 3.0)]);
    let no_lights = LightLayer {
      lights: None,
      z: 0,
      surface: LightSurface::Sequence,
      behind: Vec::new(),
      layer: solid(),
    };
    let scene = Scene::new(vec![
      LightLayer::area(LightQ::every(), "Nowhere", solid()).into(),
      no_lights.into(),
    ]);
    let placement = LightPlacement::new(&scene, &context.base.lights, DEFAULT_LIGHT_RADIUS);
    assert!(
      placement
        .layers()
        .iter()
        .all(|placed| placed.placed.is_empty())
    );
    assert_eq!(placement.layers()[0].plane, None);
  }

  #[test]
  fn a_graph_layer_numbers_its_lights_as_nodes() {
    let context = placed_lights(&[at(1.0, 5.0), at(2.0, 5.0), at(3.0, 5.0)]);
    let scene = Scene::new(vec![
      LightLayer::area(LightQ::every(), "Plane", graph_layer()).into(),
    ]);
    let placement = LightPlacement::new(&scene, &context.base.lights, DEFAULT_LIGHT_RADIUS);
    let placed = &placement.layers()[0];
    assert_eq!(placed.graph.as_ref().unwrap().nodes().len(), 3);
    assert_eq!(
      placed
        .placed
        .iter()
        .map(|light| light.at.node)
        .collect::<Vec<_>>(),
      [Some(0), Some(1), Some(2)]
    );
  }

  #[test]
  fn the_same_lights_share_a_graph() {
    let context = placed_lights(&[at(1.0, 5.0), at(2.0, 5.0), at(3.0, 5.0), at(4.0, 5.0)]);
    let graph = || {
      let every = LightQ::every();
      let placed = place_lights(
        scene::Surface::Area,
        &context.base.lights,
        Some(&PLANE),
        &every,
        DEFAULT_LIGHT_RADIUS,
      );
      place_graph(placed, Some(&PLANE)).1
    };
    assert!(Arc::ptr_eq(&graph(), &graph()));
    assert_eq!(graph().edges().count(), 3);
  }
}
