use frontbox_gfx::scene::{Covering, LayerFrame, Sample, Shape};
use serde::{Deserialize, Serialize};

use crate::{Scene, SceneLayer};

/// Part of a layer that's drawn behind something, rather than at the layer's own place: a root going behind the post it
/// wraps around.
///
/// Wherever `area` covers, the layer is drawn where `under` says instead. What it goes behind only hides it where
/// that covers it, so the area doesn't have to follow an edge. Where two of a layer's areas overlap, it's drawn at the
/// lower of their places.
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behind {
  pub under: Under,
  /// Where it does, in the units of what the layer is drawn on. Only where it covers counts: not its strokes or
  /// shadows.
  pub area: Shape,
}

impl Behind {
  /// Behind the nearest layer named `layer` under it in its scene
  pub fn new(layer: impl Into<String>, area: Shape) -> Self {
    Self {
      under: Under::Layer(layer.into()),
      area,
    }
  }

  /// Down at `z`, behind everything above that, whichever scene it's from
  pub fn at_z(z: i8, area: Shape) -> Self {
    Self {
      under: Under::Z(z),
      area,
    }
  }
}

/// What part of a layer goes behind
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Under {
  /// The nearest layer of this name under it in its scene, on the same thing at the same z. The part is drawn just
  /// under that layer, so that one and everything between the two are over it.
  Layer(String),
  /// Everything above this z, whichever scene it's from, which is how to go behind something in another scene. The part
  /// is drawn at this z, over its own scene's layers there. It has to be lower than the layer's own z.
  Z(i8),
}

/// One step of drawing a scene's layers, from [`Scene::draw_order`]: a layer, or the part of one that's drawn behind
/// something
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrawStep {
  /// Which of the scene's layers is drawn
  pub layer: usize,
  /// Where it's drawn in this step, for the part of it that goes behind something. `None` is the rest of it, at its
  /// own place.
  pub under: Option<StepPlace>,
}

/// Where a [`DrawStep`] draws the part of a layer that goes behind something
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepPlace {
  /// Just under this one of the scene's layers
  Layer(usize),
  /// At this z, over the scene's own layers there
  Z(i8),
}

/// Where a [`DrawStep`] draws its layer in one frame, from [`Scene::drawn_part`]
pub struct DrawnPart {
  under: Option<Depth>,
  /// Each of the layer's areas that go behind something, with how far down that is
  areas: Vec<(Depth, Covering)>,
}

impl DrawnPart {
  /// Whether the step draws its layer at a sample. Every sample is in one step of a layer.
  pub fn covers(&self, at: &Sample) -> bool {
    if self.areas.is_empty() {
      return true;
    }
    let covering = self.areas.iter().filter(|(_, area)| area.covers(at));
    covering.map(|(depth, _)| *depth).min() == self.under
  }
}

/// How far down a [`StepPlace`] is, lower first: its z, then where it is among the scene's layers there
type Depth = (i8, usize);

impl Scene {
  /// The order its layers are drawn in, bottom first: by z, and in the order they're in on one z, but for the parts of
  /// them that go [`Behind`] something. One behind a layer is drawn just before that layer, and one at a z after the
  /// scene's layers there. A sound layer has a step too, to be skipped like it is among the layers.
  ///
  /// Draw each step's layer where its [`Scene::drawn_part`] covers, at its [`Scene::step_z`]. Layers on different
  /// things, and the layers of other scenes, are still for whatever draws them to tell apart.
  pub fn draw_order(&self) -> Vec<DrawStep> {
    let places: Vec<Vec<StepPlace>> = (0..self.layers.len())
      .map(|index| {
        let mut places: Vec<StepPlace> = Vec::new();
        for (place, _) in self.behind(index) {
          if !places.contains(&place) {
            places.push(place);
          }
        }
        places
      })
      .collect();
    let mut steps = Vec::with_capacity(self.layers.len());
    for index in 0..self.layers.len() {
      // a lower layer's part first, so two layers behind the same one stay in their order
      let parts = places
        .iter()
        .enumerate()
        .filter(|(_, places)| places.contains(&StepPlace::Layer(index)));
      steps.extend(parts.map(|(layer, _)| DrawStep {
        layer,
        under: Some(StepPlace::Layer(index)),
      }));
      steps.push(DrawStep {
        layer: index,
        under: None,
      });
    }
    for (layer, places) in places.iter().enumerate() {
      let down = places
        .iter()
        .filter(|place| matches!(place, StepPlace::Z(_)));
      steps.extend(down.map(|place| DrawStep {
        layer,
        under: Some(*place),
      }));
    }
    // stable, so on one z they stay in that order
    steps.sort_by_key(|step| self.step_z(*step));
    steps
  }

  /// The z `step` draws its layer at: the layer's own, but for a part that goes down to another. `None` for a sound
  /// layer.
  pub fn step_z(&self, step: DrawStep) -> Option<i8> {
    match step.under {
      Some(StepPlace::Z(z)) => Some(z),
      _ => self.layers.get(step.layer)?.z(),
    }
  }

  /// Where `step` draws its layer in `frame`, the one the layer is sampled with, for samples of `radius`
  pub fn drawn_part(&self, step: DrawStep, frame: &LayerFrame, radius: f32) -> DrawnPart {
    DrawnPart {
      under: step.under.map(|place| self.depth(place)),
      areas: self
        .behind(step.layer)
        .map(|(place, behind)| (self.depth(place), behind.area.covering(frame, radius)))
        .collect(),
    }
  }

  fn depth(&self, place: StepPlace) -> Depth {
    match place {
      StepPlace::Layer(index) => {
        let z = self.layers.get(index).and_then(SceneLayer::z);
        (z.unwrap_or_default(), index)
      }
      StepPlace::Z(z) => (z, usize::MAX),
    }
  }

  /// Each part of the layer at `index` that goes behind something, with where it's drawn. One that can't be is left
  /// out; [`Scene::check`] reports it.
  fn behind(&self, index: usize) -> impl Iterator<Item = (StepPlace, &Behind)> {
    let parts = self.layers.get(index).map_or(&[][..], SceneLayer::behind);
    parts
      .iter()
      .filter_map(move |behind| Some((self.place_under(index, &behind.under)?, behind)))
  }

  /// Where a part of the layer at `index` that goes behind `under` is drawn. `None` for a layer that isn't under it on
  /// the same thing at the same z, or a z that isn't lower than its own.
  pub(crate) fn place_under(&self, index: usize, under: &Under) -> Option<StepPlace> {
    let over = self.layers.get(index)?;
    match under {
      Under::Layer(name) => {
        let found = self.layers[..index].iter().rposition(|layer| {
          layer.layer().and_then(|drawn| drawn.name.as_deref()) == Some(name)
            && layer.is_drawn_with(over)
        });
        found.map(StepPlace::Layer)
      }
      Under::Z(z) => (*z < over.z()?).then_some(StepPlace::Z(*z)),
    }
  }
}

#[cfg(test)]
mod tests {
  use frontbox_core::prelude::Vec2;
  use frontbox_gfx::prelude::*;
  use frontbox_gfx::scene::*;

  use super::*;
  use crate::{DisplayLayer, Moment};

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
  const SIZE: Vec2 = Vec2::new(8.0, 2.0);

  fn solid(name: &str, color: Rgba<u8>, region: Shape) -> DisplayLayer {
    let fill = Fill::new(FillNode::Solid(Prop::Fixed(color.into())));
    DisplayLayer::new("Dmd", Layer::new(fill).name(name).region(region))
  }

  fn rect(x: f32, width: f32) -> Shape {
    Shape::new(ShapeNode::Rect {
      width: Prop::Fixed(width),
      height: Prop::Fixed(2.0),
    })
    .transform(Transform::Translate(Prop::Fixed(Vec2::new(x, 1.0))))
  }

  /// A red post over the middle four of eight columns, and a blue root over all of them that goes behind it in
  /// columns 2 to 5
  fn wrapped() -> Scene {
    let post = solid("Post", RED, rect(4.0, 4.0));
    let root = solid("Root", BLUE, rect(4.0, 8.0)).behind(Behind::new("Post", rect(4.0, 4.0)));
    Scene::new(vec![post.into(), root.into()])
  }

  /// The top row of the scene, drawn the way a display draws it
  fn row(scene: &Scene) -> Vec<Rgba<u8>> {
    let variables = Variables::new();
    let moment = Moment {
      time: 0.0,
      playback_seed: 0,
      variables: &variables,
    };
    let frame = scene.frame(&moment, Surface::Area, SIZE, None);
    let mut pixels = vec![Rgba([0, 0, 0, 0]); 8];
    for step in scene.draw_order() {
      let Some(layer) = scene.layers[step.layer].layer() else {
        continue;
      };
      if step.under.is_none() {
        assert_eq!(scene.step_z(step), scene.layers[step.layer].z());
      }
      let prepared = layer.prepare(&frame, 0.0);
      let part = scene.drawn_part(step, &frame, 0.0);
      for (x, pixel) in pixels.iter_mut().enumerate() {
        let at = Sample::new(Vec2::new(x as f32 + 0.5, 0.5), 0.0);
        if part.covers(&at) {
          *pixel = prepared.composite(&at, *pixel);
        }
      }
    }
    pixels
  }

  #[test]
  fn a_part_behind_a_layer_is_drawn_just_before_it() {
    assert_eq!(
      wrapped().draw_order(),
      vec![
        DrawStep {
          layer: 1,
          under: Some(StepPlace::Layer(0))
        },
        DrawStep {
          layer: 0,
          under: None
        },
        DrawStep {
          layer: 1,
          under: None
        },
      ]
    );
  }

  #[test]
  fn the_layer_it_goes_behind_hides_it_where_its_area_covers() {
    assert_eq!(
      row(&wrapped()),
      vec![BLUE, BLUE, RED, RED, RED, RED, BLUE, BLUE]
    );
  }

  #[test]
  fn it_still_shows_in_its_area_where_the_other_layer_isnt() {
    let post = solid("Post", RED, rect(4.0, 2.0));
    let root = solid("Root", BLUE, rect(4.0, 8.0)).behind(Behind::new("Post", rect(4.0, 6.0)));
    let scene = Scene::new(vec![post.into(), root.into()]);
    assert_eq!(
      row(&scene),
      vec![BLUE, BLUE, BLUE, RED, RED, BLUE, BLUE, BLUE]
    );
  }

  #[test]
  fn it_goes_behind_everything_between_the_two() {
    let post = solid("Post", RED, rect(1.0, 2.0));
    let green = Rgba([0, 255, 0, 255]);
    let vines = solid("Vines", green, rect(5.0, 2.0));
    let root = solid("Root", BLUE, rect(4.0, 8.0)).behind(Behind::new("Post", rect(4.0, 8.0)));
    let scene = Scene::new(vec![post.into(), vines.into(), root.into()]);
    assert_eq!(
      row(&scene),
      vec![RED, RED, BLUE, BLUE, green, green, BLUE, BLUE]
    );
  }

  #[test]
  fn where_two_areas_overlap_it_goes_behind_the_lower_layer() {
    let post = solid("Post", RED, rect(4.0, 8.0));
    let green = Rgba([0, 255, 0, 255]);
    let vines = solid("Vines", green, rect(6.0, 4.0));
    let root = solid("Root", BLUE, rect(4.0, 8.0))
      .behind(Behind::new("Vines", rect(6.0, 4.0)))
      .behind(Behind::new("Post", rect(7.0, 2.0)));
    let scene = Scene::new(vec![post.into(), vines.into(), root.into()]);
    // behind the vines only in columns 4 and 5, where the post is still under it
    assert_eq!(
      row(&scene),
      vec![BLUE, BLUE, BLUE, BLUE, green, green, green, green]
    );
    assert_eq!(scene.draw_order().len(), 5);
  }

  #[test]
  fn a_layer_it_names_has_to_be_under_it_on_the_same_thing_at_the_same_z() {
    let behind = |post: DisplayLayer, root: DisplayLayer| {
      let root = root.behind(Behind::new("Post", rect(4.0, 4.0)));
      Scene::new(vec![post.into(), root.into()]).check()
    };
    let post = || solid("Post", RED, rect(4.0, 4.0));
    let root = || solid("Root", BLUE, rect(4.0, 8.0));
    assert!(behind(post(), root()).is_ok());
    let wrong = [
      behind(solid("Pole", RED, rect(4.0, 4.0)), root()),
      behind(post().at_z(1), root()),
      behind(
        DisplayLayer {
          display: "Other".into(),
          ..post()
        },
        root(),
      ),
    ];
    for result in wrong {
      assert!(matches!(result, Err(SceneError::NoLayerBehind { .. })));
    }
    // over it rather than under it
    let over = root().behind(Behind::new("Post", rect(4.0, 4.0)));
    let scene = Scene::new(vec![over.into(), post().into()]);
    assert!(matches!(
      scene.check(),
      Err(SceneError::NoLayerBehind { .. })
    ));
    assert_eq!(scene.draw_order().len(), 2);
  }

  #[test]
  fn a_part_at_a_lower_z_is_drawn_there_over_its_scenes_layers() {
    let ground = solid("Ground", RED, rect(1.0, 2.0));
    let green = Rgba([0, 255, 0, 255]);
    let post = solid("Post", green, rect(4.0, 2.0)).at_z(1);
    let root = solid("Root", BLUE, rect(4.0, 8.0))
      .at_z(2)
      .behind(Behind::at_z(0, rect(3.0, 4.0)));
    let scene = Scene::new(vec![ground.into(), post.into(), root.into()]);
    scene.check().unwrap();
    let order = scene.draw_order();
    let part = DrawStep {
      layer: 2,
      under: Some(StepPlace::Z(0)),
    };
    assert_eq!(order[1], part);
    assert_eq!(scene.step_z(part), Some(0));
    // down at z 0 in columns 1 to 4: over the ground in column 1, and under the post at z 1 in columns 3 and 4
    assert_eq!(
      row(&scene),
      vec![BLUE, BLUE, BLUE, green, green, BLUE, BLUE, BLUE]
    );
  }

  #[test]
  fn a_z_it_goes_down_to_has_to_be_lower_than_its_own() {
    let at = |z: i8| {
      let root = solid("Root", BLUE, rect(4.0, 8.0)).behind(Behind::at_z(z, rect(3.0, 4.0)));
      Scene::new(vec![root.into()]).check()
    };
    assert!(at(-1).is_ok());
    assert!(matches!(at(0), Err(SceneError::NotBehindZ { .. })));
    assert!(matches!(at(3), Err(SceneError::NotBehindZ { .. })));
  }

  #[test]
  fn it_reads_the_same_from_a_file() {
    let scene = Scene::load(&wrapped().save().unwrap()).unwrap();
    assert_eq!(scene.layers[1].behind().len(), 1);
    assert_eq!(row(&scene), row(&wrapped()));
    // and a layer with none doesn't say so
    assert_eq!(wrapped().save().unwrap().matches("behind").count(), 1);
  }
}
