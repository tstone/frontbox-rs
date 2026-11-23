use frontbox_core::prelude::{LightLookup, LightQ, Rgb, Vec2};
use frontbox_gfx::layout::{Grid, Line};
use frontbox_gfx::prelude::{Rgba, RgbaColor};
use frontbox_gfx::scene::{
  Bounds, Content, Graph, LayerFrame, PreparedLayer, Sample, ShapePose, Surface, Timing, Variables,
};
use frontbox_lighting::{LightPlacement, PlacedLayer, led_color};
use frontbox_scene::{Moment, Scene, SceneLayer};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::protocol::{
  ClipCanvas, ClipSound, ReferenceScene, RenderedArea, RenderedCanvas, RenderedClip, RenderedGraph,
  RenderedScene, RenderedShape, RenderedShift, SceneSize, SceneSource,
};

/// The most points one render will sample. A render costs the machine time and anything on the network can ask for
/// one. This is far more than any DMD or list of lights.
const MAX_SAMPLES: usize = 1 << 18;

/// The most points one clip will sample, across all of its frames. About 30 MB of colors.
const MAX_CLIP_SAMPLES: usize = 1 << 23;

const CLIP_FRAMES_PER_SECOND: f32 = 30.0;

/// The machine a scene is previewed on
pub(crate) struct PreviewMachine {
  pub lights: LightLookup,
  pub radius: f32,
  /// Each display's width and height in pixels, by the name it was added under
  pub displays: BTreeMap<String, (usize, usize)>,
}

impl PreviewMachine {
  /// A display's width and height, or for one the machine doesn't have, the size asked for
  fn display_size(&self, display: &str, size: SceneSize) -> (usize, usize) {
    self
      .displays
      .get(display)
      .copied()
      .unwrap_or((size.width, size.height))
  }
}

/// Renders a moment of a scene with its default variables, on its strip and each display it draws on. Without a cycle,
/// that moment is in one from the middle of the scene. The reply has the scene in both forms, as this is the only place
/// RON is read or written for a client. The error is for showing to whoever wrote the scene.
pub(crate) fn render_scene(
  source: SceneSource,
  size: SceneSize,
  machine: PreviewMachine,
  content: Content,
  time: f32,
  cycle: Option<u32>,
  reference: Option<ReferenceScene>,
  seed: u32,
) -> Result<RenderedScene, String> {
  let (scene, preview) = read(source)?;
  let drawn = &with_assets(preview.unwrap_or_else(|| scene.clone()), &content)?;
  // one that can't be drawn is left out, as the scene being edited still can be
  let (reference, reference_error) =
    match reference.map(|reference| Reference::open(reference, size, &machine, &content)) {
      Some(Ok(reference)) => (Some(reference), None),
      Some(Err(error)) => (None, Some(error)),
      None => (None, None),
    };
  let canvases = Canvas::all(
    drawn,
    reference.as_ref().map(|reference| &reference.scene),
    size,
    &machine,
  )?;
  let mut reference = reference.map(|reference| reference.drawn_on(&canvases));
  let raw_time = time;
  let (shown, time) = match cycle {
    Some(cycle) => (
      drawn.clone(),
      drawn
        .timing
        .map_or(time, |timing| timing.cycle_time(cycle, time)),
    ),
    None => {
      let middle = Scene {
        timing: drawn.timing.map(Timing::middle_cycle),
        ..drawn.clone()
      };
      let time = middle
        .timing
        .map_or(time, |timing| timing.middle_cycle_time(time));
      (middle, time)
    }
  };
  let variables = Variables::new();
  let moment = Moment {
    time,
    playback_seed: seed,
    variables: &variables,
  };
  let mut views = Views::of(&shown, size, &machine);
  views.placement.update_visits(&shown, &moment);
  // the middle of a scene that repeats is a few cycles in, which a reference scene played alongside it isn't
  let reference_moment = Moment {
    time: if cycle.is_some() { time } else { raw_time },
    ..moment
  };
  if let Some(reference) = &mut reference {
    reference
      .views
      .placement
      .update_visits(&reference.scene, &reference_moment);
  }

  let canvases = canvases.iter().enumerate().map(|(index, canvas)| {
    let other = reference
      .as_ref()
      .and_then(|reference| reference.stacked(index, &reference_moment));
    let colors = canvas.colors(&shown, &views, &moment, other).collect();
    RenderedCanvas {
      display: canvas.display.clone(),
      width: canvas.width,
      height: canvas.height,
      colors,
      area: rendered_area(canvas.covers),
    }
  });
  let canvases = canvases.collect();
  // a sound layer draws nothing, so has no outlines or shapes
  let frames = || {
    (0..shown.layers.len()).map(|index| {
      Some((
        shown.layers[index].layer()?,
        views.frame(&shown, index, &moment),
      ))
    })
  };
  let outlines = frames()
    .map(|drawn| {
      drawn.map_or(Vec::new(), |(layer, frame)| {
        rendered_outline(&layer.outline(&frame))
      })
    })
    .collect();
  let shapes = rendered_shapes(
    frames().map(|drawn| drawn.and_then(|(layer, frame)| layer.shape_pose(&frame))),
  );
  let mask_outlines = frames()
    .map(|drawn| {
      drawn.map_or(Vec::new(), |(layer, frame)| {
        rendered_outline(&layer.mask_outline(&frame))
      })
    })
    .collect();
  let mask_shapes =
    rendered_shapes(frames().map(|drawn| drawn.and_then(|(layer, frame)| layer.mask_pose(&frame))));
  let light_colors = light_colors(
    &shown,
    &views,
    &moment,
    reference
      .as_ref()
      .map(|reference| (reference, &reference_moment)),
  );
  let graphs = (0..shown.layers.len())
    .map(|index| views.placed(index).and_then(rendered_graph))
    .collect();
  let ron = scene.save().map_err(|err| err.to_string())?;

  Ok(RenderedScene {
    scene,
    ron,
    canvases,
    outlines,
    shapes,
    mask_outlines,
    mask_shapes,
    light_colors,
    graphs,
    reference_error,
  })
}

/// Renders every frame of one cycle of a scene, for a client to play without coming back to the machine until the next.
/// See [`RenderedClip`] for how it's laid out.
pub(crate) fn render_clip(
  id: u32,
  source: SceneSource,
  size: SceneSize,
  machine: PreviewMachine,
  content: Content,
  cycle: u32,
  reference: Option<ReferenceScene>,
  seed: u32,
) -> Result<Vec<u8>, String> {
  let (scene, preview) = read(source)?;
  let scene = with_assets(preview.unwrap_or(scene), &content)?;
  let Some(timing) = scene.timing else {
    return Err("a static scene has nothing to play".into());
  };
  // one that can't be drawn is left out, as a render of the moment says why
  let reference =
    reference.and_then(|reference| Reference::open(reference, size, &machine, &content).ok());
  let canvases = Canvas::all(
    &scene,
    reference.as_ref().map(|reference| &reference.scene),
    size,
    &machine,
  )?;
  let mut reference = reference.map(|reference| reference.drawn_on(&canvases));
  let mut views = Views::of(&scene, size, &machine);
  let mut lights = views.light_names();
  lights.extend(
    reference
      .iter()
      .flat_map(|reference| reference.views.light_names()),
  );
  // a frame at each end, so that a clip finishes on the scene's last moment
  let frames = ((timing.duration * CLIP_FRAMES_PER_SECOND).ceil() as usize).saturating_add(1);
  // no more than a render samples, so this can't overflow
  let points: usize = canvases
    .iter()
    .map(|canvas| canvas.width * canvas.height)
    .sum();
  let total = points.saturating_add(lights.len()).saturating_mul(frames);
  if total > MAX_CLIP_SAMPLES {
    return Err(format!(
      "this clip is {total} points, more than the {MAX_CLIP_SAMPLES} one can sample"
    ));
  }

  let light_count = lights.len();
  let variables = Variables::new();
  let sounds = scene.sounds_in_cycle(cycle).into_iter();
  let sounds = sounds.map(|start| ClipSound {
    at: start.time.rem_euclid(timing.duration),
    sound: start.sound.into(),
    volume: start.volume,
    skip: start.skip,
    length: start.length,
  });
  let sounds = sounds.collect();
  let clip_canvases = canvases.iter().map(|canvas| ClipCanvas {
    display: canvas.display.clone(),
    width: canvas.width,
    height: canvas.height,
  });
  let header = RenderedClip {
    id,
    frames,
    canvases: clip_canvases.collect(),
    lights: lights.iter().cloned().collect(),
    sounds,
  };
  let header = serde_json::to_vec(&header).map_err(|err| err.to_string())?;

  let mut clip = Vec::with_capacity(4 + header.len() + frames * (points * 4 + light_count * 3));
  clip.extend((header.len() as u32).to_le_bytes());
  clip.extend(header);

  // each canvas's frames go together
  let mut drawn: Vec<Vec<u8>> = canvases
    .iter()
    .map(|canvas| Vec::with_capacity(frames * canvas.width * canvas.height * 4))
    .collect();
  let mut lights_shown = Vec::with_capacity(frames * light_count * 3);
  for index in 0..frames {
    let time = timing.cycle_time(cycle, timing.duration * index as f32 / (frames - 1) as f32);
    let moment = Moment {
      time,
      playback_seed: seed,
      variables: &variables,
    };
    // a clip is one cycle, so its traversals are the same in every frame
    if index == 0 {
      views.placement.update_visits(&scene, &moment);
      if let Some(reference) = &mut reference {
        reference
          .views
          .placement
          .update_visits(&reference.scene, &moment);
      }
    }
    for (canvas_index, canvas) in canvases.iter().enumerate() {
      let other = reference
        .as_ref()
        .and_then(|reference| reference.stacked(canvas_index, &moment));
      drawn[canvas_index].extend(canvas.colors(&scene, &views, &moment, other));
    }
    // every light is shown every frame, and in name order, as `lights` has them
    let shown = light_colors(
      &scene,
      &views,
      &moment,
      reference.as_ref().map(|reference| (reference, &moment)),
    );
    lights_shown.extend(
      lights
        .iter()
        .map(|name| shown.get(name).copied().unwrap_or(Rgb { r: 0, g: 0, b: 0 }))
        .flat_map(|Rgb { r, g, b }| [r, g, b]),
    );
  }
  clip.extend(drawn.into_iter().flatten());
  clip.extend(lights_shown);
  Ok(clip)
}

/// A scene as it's written to a file, which has to be one that can be played. RON is kept as it was typed, comments and
/// all.
pub(crate) fn scene_file(source: SceneSource) -> Result<String, String> {
  match source {
    SceneSource::Ron(ron) => {
      Scene::load(&ron).map_err(|err| err.to_string())?;
      Ok(ron)
    }
    SceneSource::Definition(scene) | SceneSource::Preview { scene, .. } => {
      checked(scene)?.save().map_err(|err| err.to_string())
    }
  }
}

/// The scene, and what to draw in its place if it's only being previewed
fn read(source: SceneSource) -> Result<(Scene, Option<Scene>), String> {
  match source {
    SceneSource::Ron(ron) => Ok((Scene::load(&ron).map_err(|err| err.to_string())?, None)),
    SceneSource::Definition(scene) => Ok((checked(scene)?, None)),
    SceneSource::Preview { scene, shown } => Ok((checked(scene)?, Some(checked(shown)?))),
  }
}

fn checked(scene: Scene) -> Result<Scene, String> {
  frontbox_gfx::scene::file::check_version(scene.version).map_err(|err| err.to_string())?;
  scene.check().map_err(|err| err.to_string())?;
  Ok(scene)
}

/// The scene with the images and fonts it draws. Each is only loaded the first time, so this is cheap for every frame the
/// client asks for.
fn with_assets(scene: Scene, content: &Content) -> Result<Scene, String> {
  scene.with_assets(content).map_err(|err| err.to_string())
}

/// What each light shows, as light management would resolve it: higher z on top, and within a z, later layers on top.
/// A reference scene's layers are stacked in by z, under the scene's on the same z, as a scene that started playing first
/// is on the machine.
fn light_colors(
  scene: &Scene,
  views: &Views,
  moment: &Moment,
  reference: Option<(&Reference, &Moment)>,
) -> BTreeMap<String, Rgb> {
  let mut layers = reference.map_or(Vec::new(), |(reference, moment)| {
    reference.views.placement.colors(&reference.scene, moment)
  });
  layers.extend(views.placement.colors(scene, moment));
  // stable, so on one z the reference's stay under the scene's
  layers.sort_by_key(|(z, _, _)| *z);
  let mut shown: BTreeMap<String, Rgba<u8>> = BTreeMap::new();
  for (_, light, color) in layers {
    shown
      .entry(light.name.clone())
      .and_modify(|below| *below = color.composite_over(*below))
      .or_insert(color);
  }
  shown
    .into_iter()
    .map(|(name, color)| (name, led_color(color)))
    .collect()
}

/// A scene from the content folder drawn with the one being rendered, as a [`ReferenceScene`] asks
struct Reference {
  scene: Scene,
  views: Views,
  /// Its own drawing of each of the rendered scene's canvases. `None` where it doesn't draw on the same points, as on a
  /// strip of a different number of rows.
  canvases: Vec<Option<Canvas>>,
}

impl Reference {
  fn open(
    reference: ReferenceScene,
    size: SceneSize,
    machine: &PreviewMachine,
    content: &Content,
  ) -> Result<Self, String> {
    let failed = |err: String| format!("{} can't be shown with this scene: {err}", reference.name);
    let source = content
      .read_scene(&reference.name)
      .map_err(|err| failed(err.to_string()))?;
    let scene = Scene::load(&source).map_err(|err| failed(err.to_string()))?;
    let scene = with_assets(scene, content).map_err(failed)?;
    let views = Views::of(&scene, size, machine);
    Ok(Self {
      scene,
      views,
      canvases: Vec::new(),
    })
  }

  /// Draws on the same points as each of `rendered` where it can: the same display, or a strip with as many rows
  fn drawn_on(self, rendered: &[Canvas]) -> Self {
    let canvases = rendered.iter().map(|canvas| match &canvas.display {
      Some(display) => Some(Canvas::display(
        &self.scene,
        display,
        (canvas.width, canvas.height),
      )),
      None => {
        Canvas::strip(&self.scene, canvas.width).filter(|strip| strip.height == canvas.height)
      }
    });
    Self {
      canvases: canvases.collect(),
      ..self
    }
  }

  /// Its drawing of the rendered scene's canvas at `index` at `moment`, where it draws on the same points
  fn stacked<'a>(&'a self, index: usize, moment: &'a Moment<'a>) -> Option<Stacked<'a>> {
    Some(Stacked {
      canvas: self.canvases[index].as_ref()?,
      scene: &self.scene,
      views: &self.views,
      moment,
    })
  }
}

/// What each layer of a scene is drawn on in a render
struct Views {
  /// Its light layers on the machine's lights. One with no lights chosen yet previews on every light.
  placement: LightPlacement,
  /// By the scene's layers
  layers: Vec<View>,
}

/// What a layer is drawn on
struct View {
  surface: Surface,
  /// In its own units. A light layer on a plane the machine doesn't have reaches no lights, and is the size of the render.
  size: Vec2,
  /// Where a light layer is in the placement
  placed: Option<usize>,
}

impl Views {
  fn of(scene: &Scene, size: SceneSize, machine: &PreviewMachine) -> Self {
    let render_size = Vec2::new(size.width as f32, size.height as f32);
    let mut chosen = scene.clone();
    for layer in chosen
      .layers
      .iter_mut()
      .filter_map(SceneLayer::as_lights_mut)
      .filter(|layer| layer.lights.is_none())
    {
      layer.lights = Some(LightQ::Every);
    }
    let placement = LightPlacement::new(&chosen, &machine.lights, machine.radius);
    let layers = scene
      .layers
      .iter()
      .enumerate()
      .map(|(index, layer)| match layer {
        SceneLayer::Lights(_) => {
          let placed = placement
            .layers()
            .iter()
            .position(|placed| placed.index == index);
          let on = placed.map(|placed| &placement.layers()[placed]);
          let surface = on.map_or(Surface::Area, |on| on.surface);
          View {
            surface,
            size: on
              .and_then(|on| on.plane)
              .map_or(render_size, |plane| plane.extent),
            placed,
          }
        }
        SceneLayer::Display(layer) => {
          let (width, height) = machine.display_size(&layer.display, size);
          View {
            surface: Surface::Area,
            size: Vec2::new(width as f32, height as f32),
            placed: None,
          }
        }
        SceneLayer::Sound(_) => View {
          surface: Surface::Sequence,
          size: Vec2::ZERO,
          placed: None,
        },
      });
    let layers = layers.collect();
    Self { placement, layers }
  }

  fn placed(&self, index: usize) -> Option<&PlacedLayer> {
    Some(&self.placement.layers()[self.layers[index].placed?])
  }

  /// What to sample the layer at `index` with
  fn frame<'a>(&'a self, scene: &'a Scene, index: usize, moment: &Moment<'a>) -> LayerFrame<'a> {
    let view = &self.layers[index];
    scene.frame(
      moment,
      view.surface,
      view.size,
      self.placed(index).and_then(PlacedLayer::kept_visits),
    )
  }

  /// Every light a layer is on, by name
  fn light_names(&self) -> BTreeSet<String> {
    self
      .placement
      .layers()
      .iter()
      .flat_map(|placed| placed.placed.iter().map(|light| light.name.clone()))
      .collect()
  }
}

/// What's drawn in a render apart from the lights: a display's screen, or a strip of the `Sequence` layers on lights. A light
/// layer covering an `Area` is only seen on the machine.
struct Canvas {
  /// The display, by name, or `None` for the strip
  display: Option<String>,
  width: usize,
  height: usize,
  samples: Vec<Sample>,
  /// The part of the scene the samples show
  covers: Bounds,
  /// For each row of points, which layers it draws, bottom to top
  rows: Vec<Vec<usize>>,
}

impl Canvas {
  /// The strip, if the scene has `Sequence` layers, then each display it or `reference` draws on, at the display's size
  fn all(
    scene: &Scene,
    reference: Option<&Scene>,
    size: SceneSize,
    machine: &PreviewMachine,
  ) -> Result<Vec<Self>, String> {
    let mut displays: Vec<&str> = Vec::new();
    for display in display_names(scene).chain(reference.into_iter().flat_map(display_names)) {
      if !displays.contains(&display) {
        displays.push(display);
      }
    }
    // counted before anything is sampled, as a size can be anything a client asks for
    let rows = strips(scene).len();
    let strip = if rows > 0 {
      size.lights.saturating_mul(rows)
    } else {
      0
    };
    let points = displays.iter().fold(strip, |total, display| {
      let (width, height) = machine.display_size(display, size);
      total.saturating_add(width.saturating_mul(height))
    });
    if points > MAX_SAMPLES {
      return Err(format!(
        "{points} points is more than the {MAX_SAMPLES} a render can sample"
      ));
    }
    let displays = displays
      .into_iter()
      .map(|display| Self::display(scene, display, machine.display_size(display, size)));
    Ok(
      Self::strip(scene, size.lights)
        .into_iter()
        .chain(displays)
        .collect(),
    )
  }

  /// The strip of the scene's `Sequence` layers, `lights` points long, with a row for each set of lights. `None` for a scene
  /// without any.
  fn strip(scene: &Scene, lights: usize) -> Option<Self> {
    let rows = strips(scene);
    if rows.is_empty() {
      return None;
    }
    let line = Line::new(lights);
    Some(Self {
      display: None,
      width: lights,
      height: rows.len(),
      samples: line.samples().collect(),
      covers: line.covers(),
      rows,
    })
  }

  /// A display's screen, with whichever of the scene's layers are on it
  fn display(scene: &Scene, display: &str, (width, height): (usize, usize)) -> Self {
    let grid = Grid::new(width, height, Vec2::new(width as f32, height as f32));
    let rows = vec![display_layers(scene, display)];
    Self {
      display: Some(display.into()),
      width,
      height,
      samples: grid.samples().collect(),
      covers: grid.covers(),
      rows,
    }
  }

  /// Red, green, blue, and alpha for each point, with `other`'s layers stacked in among the scene's by z. On one z,
  /// `other`'s are under the scene's, as a scene that started playing first is on the machine. Every row of a display is
  /// in the one row of `rows`.
  fn colors<'a>(
    &'a self,
    scene: &'a Scene,
    views: &'a Views,
    moment: &'a Moment,
    other: Option<Stacked<'a>>,
  ) -> impl Iterator<Item = u8> + 'a {
    let radius = self.samples.first().map_or(0.0, |at| at.radius);
    let mut rows = self.prepared(scene, views, moment, radius);
    if let Some(other) = other {
      let theirs = other
        .canvas
        .prepared(other.scene, other.views, other.moment, radius);
      for (ours, mut layers) in rows.iter_mut().zip(theirs) {
        layers.append(ours);
        // stable, so on one z theirs stay under ours, and each scene's layers stay in its order
        layers.sort_by_key(|(z, _, _)| *z);
        *ours = layers;
      }
    }
    rows.into_iter().flat_map(move |layers| {
      self.samples.iter().flat_map(move |at| {
        let color = layers
          .iter()
          .fold(Rgba([0, 0, 0, 0]), |below, (_, layer, graph)| {
            // the strip has as many points as asked for, not one per light, so each shows its nearest node
            let at = graph
              .and_then(|graph| graph.nearest(at.pos))
              .map_or(*at, |node| at.at_node(node));
            layer.sample(&at).composite_over(below)
          });
        color.0
      })
    })
  }

  /// The layers each row draws, bottom to top and ready to sample, with each one's z and the graph its points go to
  fn prepared<'a>(
    &self,
    scene: &'a Scene,
    views: &'a Views,
    moment: &'a Moment,
    radius: f32,
  ) -> Vec<Vec<(i8, PreparedLayer<'a>, Option<&'a Arc<Graph>>)>> {
    let rows = self.rows.iter().map(|row| {
      let layers = row.iter().filter_map(|&index| {
        let layer = &scene.layers[index];
        let prepared = layer
          .layer()?
          .prepare(&views.frame(scene, index, moment), radius);
        let graph = views.placed(index).and_then(|placed| placed.graph.as_ref());
        Some((layer.z().unwrap_or_default(), prepared, graph))
      });
      layers.collect()
    });
    rows.collect()
  }
}

/// Another scene's drawing of the same points as a [`Canvas`], to stack in among its layers
#[derive(Clone, Copy)]
struct Stacked<'a> {
  canvas: &'a Canvas,
  scene: &'a Scene,
  views: &'a Views,
  moment: &'a Moment<'a>,
}

/// The displays a scene draws on, in the order of their first layers, some more than once
fn display_names(scene: &Scene) -> impl Iterator<Item = &str> {
  scene
    .layers
    .iter()
    .filter_map(|layer| Some(layer.as_display()?.display.as_str()))
}

/// The layers on a display, in the order they're shown
fn display_layers(scene: &Scene, display: &str) -> Vec<usize> {
  let on = |index: &usize| {
    scene.layers[*index]
      .as_display()
      .is_some_and(|layer| layer.display == display)
  };
  let mut layers: Vec<usize> = (0..scene.layers.len()).filter(on).collect();
  layers.sort_by_key(|&index| scene.layers[index].z());
  layers
}

/// The strip's rows: one for each set of lights the `Sequence` layers are on, with the layers there in the order they're
/// shown on a light
fn strips(scene: &Scene) -> Vec<Vec<usize>> {
  let mut rows: Vec<(&Option<LightQ>, Vec<usize>)> = Vec::new();
  let sequences = scene
    .layers
    .iter()
    .enumerate()
    .filter_map(|(index, layer)| Some((index, layer.as_lights()?)));
  for (index, layer) in sequences.filter(|(_, layer)| layer.surface.plane().is_none()) {
    match rows.iter_mut().find(|(lights, _)| *lights == &layer.lights) {
      Some((_, row)) => row.push(index),
      None => rows.push((&layer.lights, vec![index])),
    }
  }
  for (_, row) in &mut rows {
    row.sort_by_key(|&index| scene.layers[index].z());
  }
  rows.into_iter().map(|(_, row)| row).collect()
}

fn rendered_area(covers: Bounds) -> RenderedArea {
  let size = covers.size();
  RenderedArea {
    x: covers.min.x,
    y: covers.min.y,
    width: size.x,
    height: size.y,
  }
}

fn rendered_outline(outline: &[Vec<Vec2>]) -> Vec<Vec<[f32; 2]>> {
  outline
    .iter()
    .map(|points| points.iter().map(|p| p.to_array()).collect())
    .collect()
}

fn rendered_graph(placed: &PlacedLayer) -> Option<RenderedGraph> {
  let graph = placed.graph.as_ref()?;
  Some(RenderedGraph {
    names: graph.nodes().iter().map(|node| node.name.clone()).collect(),
    nodes: graph
      .nodes()
      .iter()
      .map(|node| node.pos.to_array())
      .collect(),
    edges: graph.edges().map(|(a, b)| [a, b]).collect(),
    lengths: placed
      .kept_visits()
      .map_or(Vec::new(), |visits| visits.lengths()),
  })
}

fn rendered_shapes(poses: impl Iterator<Item = Option<ShapePose>>) -> Vec<Option<RenderedShape>> {
  poses
    .map(|pose| pose.as_ref().map(rendered_shape))
    .collect()
}

fn rendered_shape(pose: &ShapePose) -> RenderedShape {
  RenderedShape {
    transforms: pose
      .transforms
      .iter()
      .map(|transform| transform.to_cols_array())
      .collect(),
    min: pose.bounds.min.to_array(),
    max: pose.bounds.max.to_array(),
    points: pose.points.iter().map(|p| p.to_array()).collect(),
    handles: pose.handles.iter().map(|p| p.to_array()).collect(),
    pivots: pose.pivots.iter().map(|p| p.to_array()).collect(),
    centers: pose
      .centers
      .iter()
      .map(|center| center.map(|p| p.to_array()))
      .collect(),
    anchors: pose
      .anchors
      .iter()
      .map(|anchor| anchor.map(|p| p.to_array()))
      .collect(),
    lines: pose
      .lines
      .iter()
      .map(|line| line.map(|p| p.to_array()))
      .collect(),
    shifts: pose
      .shifts
      .iter()
      .map(|line| {
        line.map(|line| RenderedShift {
          back: line.back.to_array(),
          forward: line.forward.to_array(),
          middle: line.middle.to_array(),
        })
      })
      .collect(),
    outline: pose.outline.iter().map(|p| p.to_array()).collect(),
    paths: pose
      .paths
      .iter()
      .map(|path| path.as_ref().map(rendered_shape))
      .collect(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use frontbox_core::prelude::{
    DEFAULT_LIGHT_RADIUS, ExpAddress, LedAddress, Light, LightAddress, ReferencePlane, Vec3,
  };
  use std::sync::LazyLock;

  const RED_TO_BLUE: &str = r#"Scene(
    version: 2, layers: [Lights(surface: Sequence, layer: (fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))))],)"#;

  const SIZE: SceneSize = SceneSize {
    lights: 3,
    width: 4,
    height: 2,
  };

  fn ron(source: &str) -> SceneSource {
    SceneSource::Ron(source.into())
  }

  fn no_lights() -> PreviewMachine {
    PreviewMachine {
      lights: lights(&[]),
      radius: DEFAULT_LIGHT_RADIUS,
      displays: BTreeMap::new(),
    }
  }

  /// The one canvas a render drew on
  fn only(rendered: &RenderedScene) -> &RenderedCanvas {
    assert_eq!(rendered.canvases.len(), 1, "drawn on more than one canvas");
    &rendered.canvases[0]
  }

  /// Lights in hardware order, with these locations, on a machine with the planes here
  fn lights(locations: &[Option<Vec3>]) -> LightLookup {
    let mut lights = LightLookup::default();
    lights.add_plane(&PLANE);
    lights.add_plane(&SQUARE);
    lights.add_plane(&WIDE);
    for (i, &location) in locations.iter().enumerate() {
      let name = format!("light{i}");
      let address = LightAddress::rgb(LedAddress::new(ExpAddress::default(), 0, i as u8));
      lights.insert(
        name.clone(),
        Light {
          name,
          address,
          location,
          ..Default::default()
        },
      );
    }
    lights
  }

  static PLANE: LazyLock<ReferencePlane> = LazyLock::new(|| {
    ReferencePlane::new("Plane")
      .origin(Vec3::new(100.0, 0.0, 0.0))
      .extent(Vec2::new(10.0, 10.0))
      .build()
  });

  static SQUARE: LazyLock<ReferencePlane> = LazyLock::new(|| {
    ReferencePlane::new("Square")
      .extent(Vec2::new(10.0, 10.0))
      .build()
  });

  static WIDE: LazyLock<ReferencePlane> = LazyLock::new(|| {
    ReferencePlane::new("Wide")
      .extent(Vec2::new(100.0, 50.0))
      .build()
  });

  /// Red at x = 0, blue at x = 10
  const AREA: &str = r#"Scene(
    version: 2, layers: [Lights(surface: Area("Square"), layer: (fill: (base: Gradient(stops: [
        (at: 0.0, color: Fixed(255, 0, 0, 255)),
        (at: 1.0, color: Fixed(0, 0, 255, 255)),
      ]))))],)"#;

  fn colors(source: &str, target: PreviewMachine) -> BTreeMap<String, Rgb> {
    render_scene(
      ron(source),
      SIZE,
      target,
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .light_colors
  }

  #[test]
  fn a_sequence_spreads_over_the_lights_in_order() {
    let target = PreviewMachine {
      lights: lights(&[None, None, None]),
      ..no_lights()
    };
    let colors = colors(RED_TO_BLUE, target);
    assert_eq!(colors["light0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["light2"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn a_sequence_spreads_over_only_the_chosen_lights_in_their_order() {
    let source = RED_TO_BLUE.replace(
      "surface: Sequence,",
      r#"surface: Sequence, lights: Names(["light2", "light0"]),"#,
    );
    let target = PreviewMachine {
      lights: lights(&[None, None, None]),
      ..no_lights()
    };
    let colors = colors(&source, target);
    assert_eq!(colors.len(), 2);
    assert_eq!(colors["light2"], Rgb::new(255, 0, 0));
    assert_eq!(colors["light0"], Rgb::new(0, 0, 255));
  }

  /// Red on `ring` and blue on `center`, for a second
  fn two_layers(ring: &[&str], center: &[&str]) -> String {
    format!(
      r#"Scene(
        version: 2,
        timing: (duration: 1.0),
        layers: [
          Lights(lights: Names({ring:?}), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))),
          Lights(lights: Names({center:?}), surface: Sequence, layer: (fill: (base: Solid(Fixed(0, 0, 255, 255))))),
        ],
      )"#
    )
  }

  fn three() -> PreviewMachine {
    PreviewMachine {
      lights: lights(&[None, None, None]),
      ..no_lights()
    }
  }

  #[test]
  fn every_layer_plays_on_its_own_lights_at_once() {
    let colors = colors(&two_layers(&["light0", "light1"], &["light2"]), three());
    assert_eq!(colors["light0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["light1"], Rgb::new(255, 0, 0));
    assert_eq!(colors["light2"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn the_later_layer_is_on_top_where_they_overlap() {
    let colors = colors(&two_layers(&["light0"], &["light0"]), three());
    assert_eq!(colors["light0"], Rgb::new(0, 0, 255));
  }

  #[test]
  fn each_set_of_lights_gets_a_row_of_the_strip() {
    let rendered = render_scene(
      ron(&two_layers(&["light0"], &["light1"])),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!((only(&rendered).width, only(&rendered).height), (3, 2));
    assert_eq!(only(&rendered).colors[..4], [255, 0, 0, 255]);
    assert_eq!(only(&rendered).colors[12..16], [0, 0, 255, 255]);
  }

  #[test]
  fn a_clip_plays_every_layer_on_one_timeline() {
    let clip = render_clip(
      1,
      ron(&two_layers(&["light0"], &["light2"])),
      SIZE,
      three(),
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap();
    let header_length = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    let header: serde_json::Value = serde_json::from_slice(&clip[4..4 + header_length]).unwrap();
    assert_eq!(header["lights"], serde_json::json!(["light0", "light2"]));
    assert_eq!(header["canvases"][0]["height"], 2);
    assert_eq!(clip[clip.len() - 6..], [255, 0, 0, 0, 0, 255]);
  }

  /// Red where a wave has just been, going one edge a second from `light0`, each light fading over a step
  const WAVE: &str = r#"Scene(
    version: 2, timing: (duration: 10.0, cycle: Once), layers: [Lights(surface: Area("Square"), layer: (
      fill: (base: Solid(Fixed(255, 0, 0, 255)), adjust: [Opacity(Visit(traversal: "wave", from: 0.0, to: 1.0, smooth_out: 1.0))]),
      graph: (traversals: [(name: "wave", from: Name("light0"), algorithm: BreadthFirst(), speed: Rate(Fixed(1.0)))]),
    ))],)"#;

  #[test]
  fn a_graph_is_built_from_the_lights_a_layer_lands_on_and_traversed() {
    let at = |x: f32| Some(Vec3::new(x, 5.0, 0.0));
    let target = PreviewMachine {
      lights: lights(&[at(1.0), at(2.0), at(3.0), at(4.0)]),
      ..no_lights()
    };
    let rendered = render_scene(
      ron(WAVE),
      SIZE,
      target,
      Content::without_folder(),
      2.0,
      Some(0),
      None,
      0,
    )
    .unwrap();
    let lit: Vec<bool> = (0..4)
      .map(|i| rendered.light_colors[&format!("light{i}")] != Rgb::new(0, 0, 0))
      .collect();
    assert_eq!(lit, [false, false, true, false]);

    let graph = rendered.graphs[0].as_ref().unwrap();
    assert_eq!(graph.names, ["light0", "light1", "light2", "light3"]);
    assert_eq!(graph.nodes[1], [2.0, 5.0]);
    assert_eq!(graph.edges, [[0, 1], [1, 2], [2, 3]]);
    // three edges at one a second
    assert_eq!(graph.lengths, [Some(3.0)]);
  }

  #[test]
  fn a_layer_without_a_graph_has_none() {
    let rendered = render_scene(
      ron(AREA),
      SIZE,
      three(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert!(rendered.graphs[0].is_none());
  }

  #[test]
  fn a_strip_shows_a_graph_on_a_sequence_at_its_nearest_nodes() {
    let source = WAVE.replace(r#"Area("Square")"#, "Sequence");
    let rendered = render_scene(
      ron(&source),
      SceneSize { lights: 5, ..SIZE },
      three(),
      Content::without_folder(),
      1.0,
      Some(0),
      None,
      0,
    )
    .unwrap();
    // three lights in a chain, seen on a strip of five: points halfway between two lights show the first of them
    let alphas: Vec<u8> = only(&rendered)
      .colors
      .chunks(4)
      .map(|color| color[3])
      .collect();
    assert_eq!(alphas, [0, 0, 255, 255, 0]);
  }

  #[test]
  fn an_area_colors_lights_by_where_they_are_on_the_plane() {
    let at = |x: f32| Some(Vec3::new(100.0 + x, 5.0, 0.0));
    let target = PreviewMachine {
      lights: lights(&[at(10.0), at(0.0)]),
      ..no_lights()
    };
    let colors = colors(&AREA.replace("Square", "Plane"), target);
    assert_eq!(colors["light0"], Rgb::new(0, 0, 255));
    assert_eq!(colors["light1"], Rgb::new(255, 0, 0));
  }

  #[test]
  fn lights_past_the_edge_of_the_plane_show_the_shapes_that_reach_them() {
    let source = r#"Scene(
      version: 2, layers: [Lights(surface: Area("Square"), layer: (fill: (base: Solid(Fixed(0, 0, 255, 255))))), Lights(surface: Area("Square"), layer: (
          region: (base: Ellipse(rx: Fixed(0.5), ry: Fixed(0.5)), transform: [Scale(Fixed(4.0)), Translate(Fixed(15.0, 5.0))]),
          fill: (base: Solid(Fixed(255, 0, 0, 255))),
        ))],)"#;
    let at = |x: f32| Some(Vec3::new(x, 5.0, 0.0));
    let target = PreviewMachine {
      lights: lights(&[at(15.0), at(25.0)]),
      ..no_lights()
    };
    let colors = colors(source, target);
    assert_eq!(colors["light0"], Rgb::new(255, 0, 0));
    assert_eq!(colors["light1"], Rgb::new(0, 0, 0));
  }

  #[test]
  fn shape_edges_on_lights_fade_across_the_radius_the_machine_was_booted_with() {
    let source = r#"Scene(
      version: 2, layers: [Lights(surface: Area("Square"), layer: (
        region: (base: Ellipse(rx: Fixed(0.5), ry: Fixed(0.5)), transform: [Scale(Fixed(4.0)), Translate(Fixed(5.0, 5.0))]),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      ))],)"#;
    let brightness = |radius| {
      let target = PreviewMachine {
        lights: lights(&[Some(Vec3::new(5.0, 6.95, 0.0))]),
        radius,
        ..no_lights()
      };
      colors(source, target)["light0"].r
    };
    assert!(brightness(2.0) < brightness(DEFAULT_LIGHT_RADIUS));
  }

  #[test]
  fn a_sequence_renders_to_lights() {
    let rendered = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!((only(&rendered).width, only(&rendered).height), (3, 1));
    assert_eq!(only(&rendered).colors.len(), 12);
    assert_eq!(only(&rendered).colors[..4], [255, 0, 0, 255]);
    assert_eq!(only(&rendered).colors[8..], [0, 0, 255, 255]);
  }

  #[test]
  fn a_display_renders_to_pixels_and_covers_what_it_was_asked_for() {
    let rendered = render_scene(ron(r#"Scene(version: 2, layers: [Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(0, 0, 0, 0)))))])"#), SIZE, no_lights(), Content::without_folder(), 0.0, None, None, 0).unwrap();
    assert_eq!((only(&rendered).width, only(&rendered).height), (4, 2));
    let area = &only(&rendered).area;
    assert_eq!(
      (area.x, area.y, area.width, area.height),
      (0.0, 0.0, 4.0, 2.0)
    );
  }

  #[test]
  fn area_layers_have_no_strip() {
    let source = r#"Scene(version: 2, layers: [Lights(surface: Area("Square"), layer: (fill: (base: Solid(Fixed(1, 2, 3, 255)))))])"#;
    let rendered = render_scene(
      ron(source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert!(rendered.canvases.is_empty());
  }

  /// Red on a strip, green on the `Dmd`, and blue on the `Lcd`
  const EVERYWHERE: &str = r#"Scene(version: 2, timing: (duration: 0.1, cycle: Once), layers: [
    Lights(surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))),
    Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(0, 255, 0, 255))))),
    Display(display: "Lcd", layer: (fill: (base: Solid(Fixed(0, 0, 255, 255))))),
  ])"#;

  /// A machine with a 3 by 1 `Dmd`
  fn with_dmd() -> PreviewMachine {
    PreviewMachine {
      displays: BTreeMap::from([("Dmd".into(), (3, 1))]),
      ..no_lights()
    }
  }

  #[test]
  fn a_scene_is_drawn_on_its_strip_and_each_of_its_displays() {
    let rendered = render_scene(
      ron(EVERYWHERE),
      SIZE,
      with_dmd(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let canvases: Vec<_> = rendered
      .canvases
      .iter()
      .map(|canvas| {
        (
          canvas.display.as_deref(),
          canvas.width,
          canvas.height,
          &canvas.colors[..4],
        )
      })
      .collect();
    assert_eq!(
      canvases,
      [
        (None, 3, 1, &[255, 0, 0, 255][..]),
        (Some("Dmd"), 3, 1, &[0, 255, 0, 255]),
        (Some("Lcd"), 4, 2, &[0, 0, 255, 255])
      ]
    );
  }

  #[test]
  fn a_display_layer_is_the_size_of_its_display() {
    let source = r#"Scene(version: 2, layers: [Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(1, 2, 3, 255)))))])"#;
    let rendered = render_scene(
      ron(source),
      SIZE,
      with_dmd(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(
      rendered.outlines[0],
      [[[0.0, 0.0], [3.0, 0.0], [3.0, 1.0], [0.0, 1.0]]]
    );
  }

  #[test]
  fn a_clip_has_each_canvas_s_frames_in_turn() {
    let clip = render_clip(
      0,
      ron(EVERYWHERE),
      SIZE,
      with_dmd(),
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap();
    let (header, points) = read_clip(&clip);
    let frames = header["frames"].as_u64().unwrap() as usize;
    assert_eq!(
      header["canvases"][1],
      serde_json::json!({ "display": "Dmd", "width": 3, "height": 1 })
    );
    let (strip, rest) = points.split_at(frames * 3 * 4);
    let (dmd, lcd) = rest.split_at(frames * 3 * 4);
    assert_eq!(lcd.len(), frames * 8 * 4);
    assert!(strip.chunks_exact(4).all(|point| point == [255, 0, 0, 255]));
    assert!(dmd.chunks_exact(4).all(|point| point == [0, 255, 0, 255]));
    assert!(lcd.chunks_exact(4).all(|point| point == [0, 0, 255, 255]));
  }

  #[test]
  fn a_reference_scene_shows_on_a_display_the_scene_leaves_alone() {
    let content = Content::without_folder()
      .with_scenes([("reference.scene", solid_display("0, 0, 255, 255").as_str())]);
    let reference = ReferenceScene {
      name: "reference.scene".into(),
    };
    let rendered = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      with_dmd(),
      content,
      0.0,
      None,
      Some(reference),
      0,
    )
    .unwrap();
    let dmd = &rendered.canvases[1];
    assert_eq!(
      (dmd.display.as_deref(), &dmd.colors[..4]),
      (Some("Dmd"), &[0, 0, 255, 255][..])
    );
  }

  #[test]
  fn layers_on_the_same_lights_share_a_row_of_the_strip_in_the_order_theyre_shown() {
    let layer = |z: i8, color: &str| {
      format!(
        r#"Lights(lights: Name("light0"), z: {z}, surface: Sequence, layer: (fill: (base: Solid(Fixed({color})))))"#
      )
    };
    let source = format!(
      "Scene(version: 2, layers: [{}, {}])",
      layer(1, "255, 0, 0, 255"),
      layer(0, "0, 0, 255, 255")
    );
    let rendered = render_scene(
      ron(&source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&rendered).height, 1);
    assert_eq!(only(&rendered).colors[..4], [255, 0, 0, 255]);
  }

  #[test]
  fn a_higher_z_is_on_top_on_the_lights_whatever_the_order_of_the_layers() {
    let layer = |z: i8, color: &str| {
      format!(
        r#"Lights(lights: Name("light0"), z: {z}, surface: Sequence, layer: (fill: (base: Solid(Fixed({color})))))"#
      )
    };
    let source = format!(
      "Scene(version: 2, layers: [{}, {}])",
      layer(1, "255, 0, 0, 255"),
      layer(0, "0, 0, 255, 255")
    );
    assert_eq!(colors(&source, three())["light0"], Rgb::new(255, 0, 0));
  }

  #[test]
  fn each_layer_is_seen_on_its_own_plane() {
    let gradient = |lights: &str, plane: &str| {
      format!(
        r#"Lights(lights: Name("{lights}"), surface: Area("{plane}"), layer: (region: (base: Rect(width: Fixed(2.0), height: Fixed(2.0)), transform: [Translate(Fixed(5.0, 5.0))]), fill: (base: Solid(Fixed(255, 0, 0, 255)))))"#
      )
    };
    let source = format!(
      "Scene(version: 2, layers: [{}, {}])",
      gradient("light0", "Square"),
      gradient("light1", "Plane")
    );
    // the same spot on each plane, which is somewhere different on the machine
    let target = PreviewMachine {
      lights: lights(&[
        Some(Vec3::new(5.0, 5.0, 0.0)),
        Some(Vec3::new(105.0, 5.0, 0.0)),
      ]),
      ..no_lights()
    };
    let rendered = render_scene(
      ron(&source),
      SIZE,
      target,
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(rendered.light_colors["light0"], Rgb::new(255, 0, 0));
    assert_eq!(rendered.light_colors["light1"], Rgb::new(255, 0, 0));
    assert_eq!(rendered.outlines[0], rendered.outlines[1]);
  }

  #[test]
  fn each_layer_outlines_what_its_on() {
    let on = |plane: &str| {
      format!(
        r#"Lights(surface: Area("{plane}"), layer: (fill: (base: Solid(Fixed(1, 2, 3, 255)))))"#
      )
    };
    let source = format!(
      "Scene(version: 2, layers: [{}, {}])",
      on("Square"),
      on("Wide")
    );
    let outlines = render_scene(
      ron(&source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .outlines;
    assert_eq!(
      outlines[0],
      [[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]]
    );
    assert_eq!(
      outlines[1],
      [[[0.0, 0.0], [100.0, 0.0], [100.0, 50.0], [0.0, 50.0]]]
    );
  }

  #[test]
  fn a_layer_with_no_shape_outlines_the_plane() {
    let target = no_lights();
    let source = r#"Scene(version: 2, layers: [Lights(surface: Area("Square"), layer: (fill: (base: Solid(Fixed(1, 2, 3, 255)))))])"#;
    let outlines = render_scene(
      ron(source),
      SIZE,
      target,
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .outlines;
    assert_eq!(
      outlines,
      [[[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]]]
    );
  }

  #[test]
  fn a_shape_outlines_where_it_is_on_the_plane() {
    let target = no_lights();
    let source = r#"Scene(
      version: 2, layers: [Lights(surface: Area("Square"), layer: (
        region: (base: Polygon(points: [Fixed(0.0, 0.0), Fixed(1.0, 0.0), Fixed(0.0, 1.0)]), transform: [Translate(Fixed(2.0, 3.0))]),
        fill: (base: Solid(Fixed(1, 2, 3, 255))),
      ))],)"#;
    let outlines = render_scene(
      ron(source),
      SIZE,
      target,
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .outlines;
    assert_eq!(outlines, [[[[2.0, 3.0], [3.0, 3.0], [2.0, 4.0]]]]);
  }

  #[test]
  fn a_layer_with_no_shape_outlines_a_whole_sequence() {
    let outlines = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .outlines;
    assert_eq!(
      outlines,
      [[[[0.0, -0.5], [1.0, -0.5], [1.0, 0.5], [0.0, 0.5]]]]
    );
  }

  #[test]
  fn a_rendered_shape_has_its_transforms_at_that_moment() {
    let source = r#"Scene(
      version: 2, timing: (duration: 2.0),
      layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))), Lights(surface: Sequence, layer: (
          fill: (base: Solid(Fixed(255, 0, 0, 255))),
          region: (
            base: Rect(width: Fixed(0.5), height: Fixed(0.25)),
            transform: [Translate(Keyframes(keys: [(at: 0.0, value: (0.0, 0.0)), (at: 2.0, value: (1.0, 0.0))], cycle: Once))],
          ),
        ))],)"#;
    let shapes = render_scene(
      ron(source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      1.0,
      Some(0),
      None,
      0,
    )
    .unwrap()
    .shapes;
    assert!(shapes[0].is_none());
    let rect = shapes[1].as_ref().unwrap();
    assert_eq!(rect.transforms, [[1.0, 0.0, 0.0, 1.0, 0.5, 0.0]]);
    assert_eq!((rect.min, rect.max), ([-0.25, -0.125], [0.25, 0.125]));
  }

  #[test]
  fn a_sequences_area_gives_each_point_the_same_room() {
    let area = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .canvases
    .remove(0)
    .area;
    assert_eq!(
      (area.x, area.y, area.width, area.height),
      (-0.25, -0.5, 1.5, 1.0)
    );
  }

  #[test]
  fn grid_fits_a_display() {
    let source = r#"Scene(version: 2,
      layers: [Display(display: "Dmd", layer: (
        region: (base: Ellipse(rx: Fixed(0.5), ry: Fixed(0.5)), transform: [Scale(Fixed(3.0)), Translate(Fixed(2.5, 2.5))]),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      ))],
    )"#;
    let rendered = render_scene(
      ron(source),
      SceneSize {
        lights: 0,
        width: 5,
        height: 5,
      },
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let pixel = |x: usize, y: usize| &only(&rendered).colors[(y * 5 + x) * 4..][..4];
    assert_eq!(pixel(2, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 0), [0, 0, 0, 0]);
  }

  #[test]
  fn reports_where_a_scene_is_wrong() {
    let error = render_scene(
      ron("Scene(version: 2, layers: [Lights(surface: Sequence, layer: (fill: (base: Nope)))])"),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    );
    assert!(error.err().unwrap().contains("1:"));
  }

  #[test]
  fn refuses_a_size_too_large_to_sample() {
    let size = SceneSize {
      lights: usize::MAX,
      ..SIZE
    };
    assert!(
      render_scene(
        ron(RED_TO_BLUE),
        size,
        no_lights(),
        Content::without_folder(),
        0.0,
        None,
        None,
        0
      )
      .is_err()
    );
  }

  #[test]
  fn a_definition_renders_like_its_ron() {
    let from_ron = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let from_definition = render_scene(
      SceneSource::Definition(from_ron.scene.clone()),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&from_definition).colors, only(&from_ron).colors);
    assert_eq!(from_definition.ron, from_ron.ron);
  }

  fn solid(color: &str, timing: &str) -> Scene {
    let source = format!(
      "Scene(version: 2, {timing} layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Fixed({color})))))])"
    );
    render_scene(
      ron(&source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .scene
  }

  #[test]
  fn a_preview_draws_what_it_shows_and_gives_back_the_scene() {
    let (scene, shown) = (solid("255, 0, 0, 255", ""), solid("0, 0, 255, 255", ""));
    let from_definition = render_scene(
      SceneSource::Definition(scene.clone()),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let preview = render_scene(
      SceneSource::Preview { scene, shown },
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&preview).colors[0..3], [0, 0, 255]);
    assert_eq!(preview.ron, from_definition.ron);
  }

  #[test]
  fn a_clip_of_a_preview_is_of_what_it_shows() {
    let timing = "timing: (duration: 1.0),";
    let source = SceneSource::Preview {
      scene: solid("255, 0, 0, 255", timing),
      shown: solid("0, 0, 255, 255", timing),
    };
    let clip = render_clip(
      0,
      source,
      SIZE,
      no_lights(),
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap();
    let header = u32::from_le_bytes(clip[0..4].try_into().unwrap()) as usize;
    assert_eq!(clip[4 + header..7 + header], [0, 0, 255]);
  }

  #[test]
  fn what_a_client_keeps_on_a_track_for_itself_doesnt_reach_the_file() {
    let source = r#"Scene(
      version: 2, timing: (duration: 1.0),
      layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Keyframes(keys: [(at: 0.0, value: (9, 8, 7, 255))])))))],)"#;
    let scene = render_scene(
      ron(source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .scene;
    let mut json = serde_json::to_value(&scene).unwrap();
    json["layers"][0]["Lights"]["layer"]["fill"]["base"]["Solid"]["Keyframes"]["hidden"] =
      true.into();
    let back = render_scene(
      SceneSource::Definition(serde_json::from_value(json).unwrap()),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    );
    let back = back.unwrap();
    assert!(!back.ron.contains("hidden"), "{}", back.ron);
  }

  #[test]
  fn a_definition_is_held_to_the_same_rules_as_a_file() {
    let source = r#"Scene(
      version: 2, timing: (duration: 1.0),
      layers: [Lights(surface: Area("Square"), layer: (fill: (base: Solid(Keyframes(keys: [(at: 0.0, value: (0, 0, 0, 255))])))))],)"#;
    let mut scene = render_scene(
      ron(source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap()
    .scene;
    scene.timing = None;
    let error = render_scene(
      SceneSource::Definition(scene),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .err()
    .unwrap();
    assert!(error.contains("timing"), "{error}");
  }

  /// Goes from black to red over two seconds
  fn rising(cycle: &str) -> SceneSource {
    ron(&format!(
      r#"Scene(
        version: 2, timing: (duration: 2.0, cycle: {cycle}),
        layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Keyframes(keys: [
          (at: 0.0, value: (0, 0, 0, 255)),
          (at: 2.0, value: (200, 0, 0, 255)),
        ])))))],)"#
    ))
  }

  /// A clip's header, and its frames' colors
  fn read_clip(clip: &[u8]) -> (serde_json::Value, &[u8]) {
    let length = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    (
      serde_json::from_slice(&clip[4..4 + length]).unwrap(),
      &clip[4 + length..],
    )
  }

  #[test]
  fn renders_any_moment() {
    let rendered = render_scene(
      rising("Once"),
      SIZE,
      no_lights(),
      Content::without_folder(),
      1.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&rendered).colors[..4], [100, 0, 0, 255]);
  }

  #[test]
  fn the_end_of_a_cycle_is_its_last_moment_however_often_it_repeats() {
    let rendered = render_scene(
      rising("Forever"),
      SIZE,
      no_lights(),
      Content::without_folder(),
      2.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&rendered).colors[..4], [200, 0, 0, 255]);
    assert!(rendered.ron.contains("Forever"), "{}", rendered.ron);
  }

  #[test]
  fn a_preview_of_a_scene_that_repeats_smooths_across_its_cycles() {
    let scene = |cycle: &str| {
      ron(&format!(
        r#"Scene(
          version: 2, timing: (duration: 1.0, cycle: {cycle}),
          layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Random(
            picks: [(at: 0.0, smooth_in: 1.0)],
            spread: Color(hue: (0.0, 360.0)),
          )))))],)"#
      ))
    };
    let colors = |source, time| {
      render_scene(
        source,
        SIZE,
        no_lights(),
        Content::without_folder(),
        time,
        None,
        None,
        0,
      )
      .unwrap()
      .canvases
      .remove(0)
      .colors
    };
    assert_ne!(
      colors(scene("Forever"), 0.25),
      colors(scene("Forever"), 0.75)
    );
    assert_eq!(colors(scene("Once"), 0.25), colors(scene("Once"), 0.75));
    assert_eq!(
      colors(scene("Times(1)"), 0.25),
      colors(scene("Times(1)"), 0.75)
    );
  }

  /// A new color at the start of each cycle of a second
  fn picking(cycle: &str, smooth_in: f32) -> SceneSource {
    ron(&format!(
      r#"Scene(
        version: 2, timing: (duration: 1.0, cycle: {cycle}),
        layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Random(
          picks: [(at: 0.0, smooth_in: {smooth_in})],
          spread: Color(hue: (0.0, 360.0)),
        )))))],)"#
    ))
  }

  #[test]
  fn a_scene_is_filed_as_what_was_typed_or_the_definition_being_kept() {
    let typed = "// the title\nScene(version: 2, layers: [])";
    assert_eq!(scene_file(SceneSource::Ron(typed.into())).unwrap(), typed);

    let scene = Scene::load(typed).unwrap();
    let shown = Scene::new(vec![]);
    let written = scene.save().unwrap();
    assert_eq!(
      scene_file(SceneSource::Preview { scene, shown }).unwrap(),
      written
    );
  }

  /// A folder with a 2 by 1 image, red on the left and blue on the right, called `art/a.png`
  fn content(folder: &str) -> Content {
    let root =
      std::env::temp_dir().join(format!("frontbox-console-{}-{folder}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("art")).unwrap();
    image::RgbaImage::from_fn(2, 1, |x, _| {
      image::Rgba(if x == 0 {
        [255, 0, 0, 255]
      } else {
        [0, 0, 255, 255]
      })
    })
    .save(root.join("art/a.png"))
    .unwrap();
    Content::new(root)
  }

  const IMAGE: &str = r#"Scene(version: 2, layers: [Display(display: "Dmd", layer: (fill: (base: Image(name: "art/a.png"))))])"#;
  const TWO_POINTS: SceneSize = SceneSize {
    lights: 0,
    width: 2,
    height: 1,
  };

  #[test]
  fn a_render_shows_a_mask_and_says_where_it_is() {
    let source = r#"Scene(version: 2,
      layers: [
        Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(0, 0, 255, 255))))),
        Display(display: "Dmd", layer: (
          fill: (base: Solid(Fixed(255, 0, 0, 255))),
          mask: (
            fill: (base: Solid(Fixed(0, 0, 0, 255))),
            region: (base: Ellipse(rx: Fixed(0.5), ry: Fixed(0.5)), transform: [Scale(Fixed(4.0)), Translate(Fixed(5.0, 2.5))]),
          ),
        )),
      ],
    )"#;
    let rendered = render_scene(
      ron(source),
      SceneSize {
        lights: 0,
        width: 10,
        height: 5,
      },
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let pixel = |x: usize, y: usize| &only(&rendered).colors[(y * 10 + x) * 4..][..4];
    assert_eq!(pixel(5, 2), [255, 0, 0, 255]);
    assert_eq!(pixel(0, 2), [0, 0, 255, 255]);
    assert!(rendered.mask_shapes[0].is_none());
    assert!(rendered.mask_shapes[1].is_some());
    assert!(rendered.mask_outlines[0].is_empty());
    assert!(!rendered.mask_outlines[1].is_empty());
  }

  #[test]
  fn a_scene_draws_its_images_from_the_content_folder() {
    let content = content("draws");
    let rendered = render_scene(
      ron(IMAGE),
      TWO_POINTS,
      no_lights(),
      content.clone(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&rendered).colors, [255, 0, 0, 255, 0, 0, 255, 255]);
    let _ = std::fs::remove_dir_all(content.root().unwrap());
  }

  #[test]
  fn a_clip_draws_its_images_too() {
    let content = content("clip");
    let source = r#"Scene(version: 2, timing: (duration: 0.1, cycle: Once), layers: [Display(display: "Dmd", layer: (fill: (base: Image(name: "art/a.png"))))])"#;
    let clip = render_clip(
      0,
      ron(source),
      TWO_POINTS,
      no_lights(),
      content.clone(),
      0,
      None,
      0,
    )
    .unwrap();
    let header = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    assert_eq!(clip[4 + header..][..8], [255, 0, 0, 255, 0, 0, 255, 255]);
    let _ = std::fs::remove_dir_all(content.root().unwrap());
  }

  #[test]
  fn a_clip_plays_through_the_frames_of_a_gif() {
    let content = content("gif");
    let mut gif = Vec::new();
    let frame = |color| {
      let delay = image::Delay::from_numer_denom_ms(100, 1);
      image::Frame::from_parts(
        image::RgbaImage::from_pixel(2, 1, image::Rgba(color)),
        0,
        0,
        delay,
      )
    };
    image::codecs::gif::GifEncoder::new(&mut gif)
      .encode_frames([frame([255, 0, 0, 255]), frame([0, 0, 255, 255])])
      .unwrap();
    content.add_image("a.gif", &gif).unwrap();

    let source = r#"Scene(version: 2, timing: (duration: 1.0, cycle: Once), layers: [Display(display: "Dmd", layer: (fill: (base: Image(
      name: "a.gif",
      frame: Keyframes(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 2.0)]),
    ))))])"#;
    let clip = render_clip(
      0,
      ron(source),
      TWO_POINTS,
      no_lights(),
      content.clone(),
      0,
      None,
      0,
    )
    .unwrap();
    let header = u32::from_le_bytes(clip[..4].try_into().unwrap()) as usize;
    let frames = &clip[4 + header..];
    assert_eq!(frames[..4], [255, 0, 0, 255]);
    assert_eq!(frames[frames.len() - 8..frames.len() - 4], [0, 0, 255, 255]);
    let _ = std::fs::remove_dir_all(content.root().unwrap());
  }

  #[test]
  fn a_missing_image_is_an_error_naming_it() {
    let content = content("missing");
    let source = IMAGE.replace("art/a.png", "art/b.png");
    let error = render_scene(
      ron(&source),
      TWO_POINTS,
      no_lights(),
      content.clone(),
      0.0,
      None,
      None,
      0,
    )
    .err()
    .unwrap();
    assert!(error.contains("art/b.png"), "{error}");
    let _ = std::fs::remove_dir_all(content.root().unwrap());
  }

  #[test]
  fn a_machine_with_no_content_folder_says_so_only_when_a_scene_needs_one() {
    let error = render_scene(
      ron(IMAGE),
      TWO_POINTS,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .err()
    .unwrap();
    assert!(error.contains("no content folder"), "{error}");
    assert!(
      render_scene(
        ron(RED_TO_BLUE),
        SIZE,
        no_lights(),
        Content::without_folder(),
        0.0,
        None,
        None,
        0
      )
      .is_ok()
    );
  }

  #[test]
  fn each_cycle_of_a_scene_has_a_clip_of_its_own() {
    let clip = |cycle| {
      render_clip(
        0,
        picking("Forever", 0.0),
        SIZE,
        no_lights(),
        Content::without_folder(),
        cycle,
        None,
        0,
      )
      .unwrap()
    };
    let first = clip(0);
    assert_eq!(first, clip(0));
    assert_ne!(read_clip(&first).1, read_clip(&clip(1)).1);
    assert_ne!(read_clip(&clip(1)).1, read_clip(&clip(2)).1);
  }

  #[test]
  fn a_seed_gives_the_same_random_choices_every_time_and_another_seed_gives_others() {
    let clip = |seed| {
      render_clip(
        0,
        picking("Once", 0.0),
        SIZE,
        no_lights(),
        Content::without_folder(),
        0,
        None,
        seed,
      )
      .unwrap()
    };
    assert_eq!(clip(1), clip(1));
    assert_ne!(read_clip(&clip(1)).1, read_clip(&clip(2)).1);
    let moment = |seed| {
      only(
        &render_scene(
          picking("Once", 0.0),
          SIZE,
          no_lights(),
          Content::without_folder(),
          0.0,
          Some(0),
          None,
          seed,
        )
        .unwrap(),
      )
      .colors
      .clone()
    };
    assert_ne!(moment(1), moment(2));
  }

  #[test]
  fn the_first_cycle_of_a_clip_has_nothing_to_smooth_from() {
    let clip = render_clip(
      0,
      picking("Forever", 1.0),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap();
    let (header, colors) = read_clip(&clip);
    let frames = header["frames"].as_u64().unwrap() as usize;
    assert_eq!(colors[..12], colors[(frames - 1) * 12..]);
  }

  #[test]
  fn a_moment_of_a_cycle_is_what_its_clip_shows() {
    let clip = render_clip(
      0,
      picking("Forever", 1.0),
      SIZE,
      no_lights(),
      Content::without_folder(),
      4,
      None,
      0,
    )
    .unwrap();
    let (header, colors) = read_clip(&clip);
    let frames = header["frames"].as_u64().unwrap() as usize;
    let middle = (frames - 1) / 2;
    let moment = render_scene(
      picking("Forever", 1.0),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.5,
      Some(4),
      None,
      0,
    )
    .unwrap();
    assert_eq!(only(&moment).colors, colors[middle * 12..][..12]);
  }

  #[test]
  fn a_clip_has_the_sounds_its_cycle_starts_and_draws_nothing_for_them() {
    let source = r#"Scene(
      version: 2, timing: (duration: 1.0, cycle: Forever), layers: [
        Lights(lights: Name("light0"), surface: Sequence, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))),
        Sound(sound: "sounds/b.wav", volume: 0.5, start: 0.6, skip: 0.5, end: 0.85),
        Sound(sound: "sounds/a.wav", volume: 0.5, start: 0.1),
      ],)"#;
    let clip = render_clip(
      0,
      ron(source),
      SIZE,
      three(),
      Content::without_folder(),
      2,
      None,
      0,
    )
    .unwrap();
    let (header, _) = read_clip(&clip);
    let sounds = header["sounds"].as_array().unwrap();
    let starts: Vec<_> = sounds
      .iter()
      .map(|sound| {
        (
          sound["sound"].as_str().unwrap(),
          sound["at"].as_f64().unwrap(),
        )
      })
      .collect();
    assert_eq!(starts.len(), 2);
    assert_eq!(starts[0].0, "sounds/a.wav");
    assert!(
      (starts[0].1 - 0.1).abs() < 1e-4 && (starts[1].1 - 0.6).abs() < 1e-4,
      "{starts:?}"
    );
    assert_eq!(sounds[0]["volume"], 0.5);
    assert_eq!(
      (&sounds[0]["skip"], &sounds[1]["skip"]),
      (&0.0.into(), &0.5.into())
    );
    assert!(
      sounds[0]["length"].is_null() && (sounds[1]["length"].as_f64().unwrap() - 0.25).abs() < 1e-4,
      "{sounds:?}"
    );

    let rendered = render_scene(
      ron(source),
      SIZE,
      three(),
      Content::without_folder(),
      0.0,
      Some(0),
      None,
      0,
    )
    .unwrap();
    assert_eq!(
      (rendered.outlines[1].len(), rendered.shapes[1].is_none()),
      (0, true)
    );
  }

  #[test]
  fn a_clip_runs_from_the_start_of_a_cycle_to_its_end() {
    let clip = render_clip(
      7,
      rising("Forever"),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap();
    let (header, colors) = read_clip(&clip);
    assert_eq!(header["id"], 7);
    assert_eq!(header["canvases"][0]["width"], 3);
    assert_eq!(header["canvases"][0]["height"], 1);

    let frames = header["frames"].as_u64().unwrap() as usize;
    assert_eq!(frames, 61);
    assert_eq!(colors.len(), frames * 3 * 4);
    assert_eq!(colors[..4], [0, 0, 0, 255]);
    assert_eq!(colors[30 * 3 * 4..][..4], [100, 0, 0, 255]);
    assert_eq!(colors[60 * 3 * 4..][..4], [200, 0, 0, 255]);
  }

  #[test]
  fn a_clip_has_the_lights_after_the_points_with_nothing_behind_them_as_off() {
    let source = r#"Scene(
      version: 2, timing: (duration: 0.1),
      layers: [Lights(surface: Sequence, layer: (fill: (base: Solid(Fixed(200, 0, 0, 128)))))],)"#;
    let target = PreviewMachine {
      lights: lights(&[None, None]),
      ..no_lights()
    };
    let clip = render_clip(
      0,
      ron(source),
      SIZE,
      target,
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap();
    let (header, colors) = read_clip(&clip);
    assert_eq!(header["lights"], serde_json::json!(["light0", "light1"]));

    let frames = header["frames"].as_u64().unwrap() as usize;
    let lights = &colors[frames * 3 * 4..];
    assert_eq!(lights.len(), frames * 2 * 3);
    assert_eq!(lights[..6], [100, 0, 0, 100, 0, 0]);
  }

  #[test]
  fn a_display_layer_is_drawn_on_its_own_screen_and_reaches_no_lights() {
    let source = r#"Scene(version: 2, layers: [Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(255, 0, 0, 255)))))])"#;
    let target = PreviewMachine {
      lights: lights(&[None]),
      ..no_lights()
    };
    let rendered = render_scene(
      ron(source),
      SIZE,
      target,
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!((only(&rendered).width, only(&rendered).height), (4, 2));
    assert_eq!(
      (only(&rendered).area.width, only(&rendered).area.height),
      (4.0, 2.0)
    );
    assert!(rendered.light_colors.is_empty());
    assert!(rendered.ron.starts_with("Scene("), "{}", rendered.ron);
  }

  #[test]
  fn a_scene_that_cant_be_played_isnt_filed() {
    assert!(scene_file(SceneSource::Ron("(oops".into())).is_err());
    assert!(
      scene_file(SceneSource::Ron(
        "Scene(version: 2, timing: (duration: 0.0), layers: [])".into()
      ))
      .is_err()
    );
  }

  #[test]
  fn a_static_scene_has_no_clip() {
    assert!(
      render_clip(
        0,
        ron(RED_TO_BLUE),
        SIZE,
        no_lights(),
        Content::without_folder(),
        0,
        None,
        0
      )
      .is_err()
    );
  }

  #[test]
  fn refuses_a_clip_too_big_to_send() {
    let size = SceneSize {
      lights: 1 << 18,
      width: 0,
      height: 0,
    };
    let error = render_clip(
      0,
      rising("Once"),
      size,
      no_lights(),
      Content::without_folder(),
      0,
      None,
      0,
    )
    .unwrap_err();
    assert!(error.contains("more than"), "{error}");
  }

  #[test]
  fn a_scene_survives_the_trip_to_a_client_and_back() {
    let source = r#"Scene(
      version: 2, variables: {"tint": Color(1, 2, 3, 4), "size": Float(2.5), "at": Vec2(0.5, 0.25)},
      
      timing: (duration: 2.0, cycle: Forever),
      layers: [Lights(surface: Area("Square"), layer: (
        region: (
          base: Polygon(points: [Fixed(0.0, 0.0), Variable("at"), Fixed(1.0, 0.0)]),
          transform: [Rotate(Keyframes(keys: [(at: 0.0, value: 0.0), (at: 1.0, value: 90.0, curve: Steps(4))], cycle: Times(2)))],
        ),
        fill: (base: Solid(Variable("tint")), adjust: [HueShift(Variable("size"))]),
      ))],)"#;
    let rendered = render_scene(
      ron(source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(
      SceneSource::Definition(serde_json::from_str(&json).unwrap()),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(back.ron, rendered.ron);
  }

  #[test]
  fn a_sweep_survives_the_editor_and_only_on_a_position() {
    let source = r#"Scene(
      version: 2, timing: (duration: 2.0, cycle: Forever),
      layers: [Lights(surface: Area("Square"), layer: (
        region: (
          base: Ellipse(rx: Fixed(1.0), ry: Fixed(1.0)),
          transform: [Translate(Sweep(path: (base: Rect(width: Fixed(4.0), height: Fixed(2.0)), transform: [Translate(Fixed(5.0, 5.0))]), position: Keyframes(keys: [(at: 0.5, value: 0.0), (at: 1.5, value: 1.0)])))],
        ),
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
      ))],)"#;
    let rendered = render_scene(
      ron(source),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let json = serde_json::to_string(&rendered.scene).unwrap();
    let back = render_scene(
      SceneSource::Definition(serde_json::from_str(&json).unwrap()),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    assert_eq!(back.ron, rendered.ron);

    let path = rendered.shapes[0].as_ref().unwrap().paths[0]
      .as_ref()
      .unwrap();
    assert_eq!(path.outline[0], [3.0, 4.0]);

    let on_a_size = json.replace(
      r#""ry":{"Fixed":1.0}"#,
      &format!(r#""ry":{}"#, sweep_of(&json)),
    );
    let error = serde_json::from_str::<Scene>(&on_a_size)
      .unwrap_err()
      .to_string();
    assert!(error.contains("only a position"), "{error}");
  }

  /// The first sweep in a scene as JSON
  fn sweep_of(json: &str) -> String {
    let scene: serde_json::Value = serde_json::from_str(json).unwrap();
    scene["layers"][0]["Lights"]["layer"]["region"]["transform"][0]["Translate"].to_string()
  }

  fn solid_display(color: &str) -> String {
    format!(
      r#"Scene(version: 2, layers: [Display(display: "Dmd", layer: (fill: (base: Solid(Fixed({color})))))])"#
    )
  }

  fn solid_display_at(z: i8, color: &str) -> String {
    format!(
      r#"Scene(version: 2, layers: [Display(display: "Dmd", z: {z}, layer: (fill: (base: Solid(Fixed({color})))))])"#
    )
  }

  fn with_reference(source: &str, reference: &str) -> Vec<u8> {
    let content = Content::without_folder().with_scenes([("reference.scene", reference)]);
    let reference = ReferenceScene {
      name: "reference.scene".into(),
    };
    render_scene(
      ron(source),
      TWO_POINTS,
      no_lights(),
      content,
      0.0,
      None,
      Some(reference),
      0,
    )
    .unwrap()
    .canvases
    .remove(0)
    .colors
  }

  #[test]
  fn a_reference_scene_layer_on_a_higher_z_is_drawn_over_the_scene() {
    let colors = with_reference(
      &solid_display_at(0, "255, 0, 0, 255"),
      &solid_display_at(1, "0, 0, 255, 255"),
    );
    assert_eq!(colors, [0, 0, 255, 255, 0, 0, 255, 255]);
  }

  #[test]
  fn a_reference_scene_layer_on_a_lower_z_shows_where_the_scene_is_clear() {
    let blue = solid_display_at(-1, "0, 0, 255, 255");
    assert_eq!(
      with_reference(&solid_display("255, 0, 0, 255"), &blue),
      [255, 0, 0, 255, 255, 0, 0, 255]
    );
    assert_eq!(
      with_reference(&solid_display("0, 0, 0, 0"), &blue),
      [0, 0, 255, 255, 0, 0, 255, 255]
    );
  }

  #[test]
  fn a_reference_scene_layer_on_the_same_z_is_under_the_scene() {
    let colors = with_reference(
      &solid_display("255, 0, 0, 255"),
      &solid_display("0, 0, 255, 255"),
    );
    assert_eq!(colors, [255, 0, 0, 255, 255, 0, 0, 255]);
  }

  #[test]
  fn a_reference_scene_layer_goes_between_the_scene_layers_by_z() {
    let scene = |top: &str| {
      format!(
        r#"Scene(version: 2, layers: [
          Display(display: "Dmd", z: 0, layer: (fill: (base: Solid(Fixed(255, 0, 0, 255))))),
          Display(display: "Dmd", z: 2, layer: (fill: (base: Solid(Fixed({top}))))),
        ])"#
      )
    };
    let blue = solid_display_at(1, "0, 0, 255, 255");
    // over the scene's bottom layer
    assert_eq!(
      with_reference(&scene("0, 0, 0, 0"), &blue),
      [0, 0, 255, 255, 0, 0, 255, 255]
    );
    // and under its top one
    assert_eq!(
      with_reference(&scene("0, 255, 0, 255"), &blue),
      [0, 255, 0, 255, 0, 255, 0, 255]
    );
  }

  #[test]
  fn a_clip_draws_its_reference_scene_in_every_frame() {
    let content = Content::without_folder()
      .with_scenes([("reference.scene", solid_display("0, 0, 255, 255").as_str())]);
    let clear = r#"Scene(version: 2, timing: (duration: 0.1, cycle: Once), layers: [Display(display: "Dmd", layer: (fill: (base: Solid(Fixed(0, 0, 0, 0)))))])"#;
    let reference = ReferenceScene {
      name: "reference.scene".into(),
    };
    let clip = render_clip(
      0,
      ron(clear),
      TWO_POINTS,
      no_lights(),
      content,
      0,
      Some(reference),
      0,
    )
    .unwrap();
    let (header, points) = read_clip(&clip);
    assert_eq!(
      points.len(),
      header["frames"].as_u64().unwrap() as usize * 8
    );
    assert!(
      points
        .chunks_exact(4)
        .all(|point| point == [0, 0, 255, 255])
    );
  }

  #[test]
  fn a_missing_reference_scene_is_left_out_and_says_why() {
    let reference = ReferenceScene {
      name: "gone.scene".into(),
    };
    let alone = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      None,
      0,
    )
    .unwrap();
    let rendered = render_scene(
      ron(RED_TO_BLUE),
      SIZE,
      no_lights(),
      Content::without_folder(),
      0.0,
      None,
      Some(reference),
      0,
    )
    .unwrap();
    assert_eq!(only(&rendered).colors, only(&alone).colors);
    let error = rendered.reference_error.unwrap();
    assert!(
      error.starts_with("gone.scene can't be shown with this scene"),
      "{error}"
    );
    assert_eq!(alone.reference_error, None);
  }
}
