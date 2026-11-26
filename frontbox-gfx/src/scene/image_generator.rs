use std::fmt;
use std::sync::Arc;

use image::RgbaImage;

use crate::scene::generator::{Kept, Moment, unknown_setting};
use crate::scene::{
  FillNode, GeneratorError, GeneratorFrame, GeneratorKind, GeneratorSetting, Picture,
};

/// Images made by code outside gfx, for [`FillNode::External`](crate::scene::FillNode::External)
///
/// A generator is for a picture too involved to build from layers, that's easier drawn pixel by pixel: a character, a
/// simulation, an effect with rules of its own. A scene names it and sets its settings; whatever loads the scene puts
/// it in the scene's [`Assets`](crate::scene::Assets) by that name. Its output is an image like any other a fill
/// draws: it fills the layer's region or keeps its own size, transforms move it, and color adjusts change it.
///
/// The image is as big as [`Self::generate`] makes it, and can be a different size from one frame to the next.
///
/// [`Self::generate`] has to be a pure function of the frame it's given, so that any moment can be sampled on its own,
/// seeking included, and its output can be kept for every sample of a frame.
pub trait ImageGenerator: Send + Sync + fmt::Debug {
  /// What a scene can set, which the editor shows. A scene's setting by any other name is an error when it loads.
  fn settings(&self) -> &[GeneratorSetting];

  /// Whether its output changes with time when its settings don't. False keeps one output for the whole playback.
  fn is_animated(&self) -> bool {
    false
  }

  /// The image at `frame`
  fn generate(&self, frame: &GeneratorFrame) -> RgbaImage;
}

/// An [`ImageGenerator`] in a scene's [`Assets`](crate::scene::Assets), which keeps its latest outputs so that every
/// sample of a frame, and every layer with the same settings, shares one call. Clones share what's kept.
#[derive(Debug, Clone)]
pub struct LoadedImageGenerator {
  generator: Arc<dyn ImageGenerator>,
  kept: Kept<Picture>,
}

impl LoadedImageGenerator {
  pub fn new(generator: Arc<dyn ImageGenerator>) -> Self {
    Self {
      generator,
      kept: Kept::default(),
    }
  }

  /// `generator` for the external fill `fill` names it in, which can only set the settings it has. `None` for a fill
  /// that isn't external.
  pub fn for_fill(
    generator: Arc<dyn ImageGenerator>,
    fill: &FillNode,
  ) -> Option<Result<Self, GeneratorError>> {
    let name = fill.generator_name()?;
    let unknown = unknown_setting(generator.settings(), fill.generator_settings());
    Some(match unknown {
      Some(setting) => Err(GeneratorError::NoSuchSetting {
        kind: GeneratorKind::Image,
        generator: name.into(),
        setting: setting.into(),
      }),
      None => Ok(Self::new(generator)),
    })
  }

  pub fn generator(&self) -> &dyn ImageGenerator {
    self.generator.as_ref()
  }

  /// The image at a moment, with `setting` giving the scene's value for a setting it sets
  pub(crate) fn generate(
    &self,
    time: f32,
    cycle: u32,
    seed: u32,
    setting: impl Fn(&str) -> Option<f32>,
  ) -> Arc<Picture> {
    let moment = Moment {
      time,
      cycle,
      seed,
      animated: self.generator.is_animated(),
    };
    self
      .kept
      .output(self.generator.settings(), moment, setting, |frame| {
        Picture::new(self.generator.generate(frame))
      })
  }
}

#[cfg(test)]
mod tests {
  use std::collections::BTreeMap;
  use std::sync::atomic::{AtomicUsize, Ordering};

  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::scene::value::tests::env;
  use crate::scene::{Assets, Env, Fill, ImageSize, Inputs, Prop, Value, Variables, file};

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);
  const CLEAR: Rgba<u8> = Rgba([0, 0, 0, 0]);

  /// An image 4 by 2, red for its first `reds` columns and blue for the rest
  #[derive(Debug, Default)]
  struct Columns {
    calls: AtomicUsize,
  }

  const SETTINGS: &[GeneratorSetting] = &[GeneratorSetting::new("reds", 2.0).range(0.0, 4.0)];

  impl ImageGenerator for Columns {
    fn settings(&self) -> &[GeneratorSetting] {
      SETTINGS
    }

    fn generate(&self, frame: &GeneratorFrame) -> RgbaImage {
      self.calls.fetch_add(1, Ordering::Relaxed);
      let reds = frame.setting("reds") as u32;
      RgbaImage::from_fn(4, 2, |x, _| if x < reds { RED } else { BLUE })
    }
  }

  fn assets() -> Assets {
    let mut assets = Assets::new();
    let columns = LoadedImageGenerator::new(Arc::new(Columns::default()));
    assets.insert_image_generator("columns", columns);
    assets
  }

  fn external(settings: &[(&str, Prop<f32>)], size: ImageSize) -> Fill {
    Fill::new(FillNode::External {
      generator: "columns".into(),
      settings: settings
        .iter()
        .map(|(name, setting)| (name.to_string(), setting.clone()))
        .collect::<BTreeMap<_, _>>(),
      size,
    })
  }

  /// The fill's color at (x, y) of a unit box `size` across in scene units
  fn color_in(
    fill: &Fill,
    assets: &Assets,
    variables: &Variables,
    size: Vec2,
    x: f32,
    y: f32,
  ) -> Rgba<u8> {
    let env = Env {
      assets,
      ..env(0.0, variables)
    };
    fill
      .prepare(Vec2::ZERO, size, &env)
      .map_or(CLEAR, |fill| fill.color(Vec2::new(x, y)))
  }

  fn color(fill: &Fill, x: f32, y: f32) -> Rgba<u8> {
    color_in(fill, &assets(), &Variables::new(), Vec2::ONE, x, y)
  }

  #[test]
  fn an_image_is_kept_for_the_same_moment_and_settings() {
    let columns = Arc::new(Columns::default());
    let loaded = LoadedImageGenerator::new(columns.clone());
    loaded.generate(1.0, 0, 0, |_| None);
    loaded.generate(2.0, 1, 0, |_| None);
    assert_eq!(columns.calls.load(Ordering::Relaxed), 1);
    loaded.generate(1.0, 0, 0, |_| Some(3.0));
    assert_eq!(columns.calls.load(Ordering::Relaxed), 2);
  }

  #[test]
  fn its_image_is_stretched_over_the_unit_box() {
    let fill = external(&[], ImageSize::Stretch);
    assert_eq!(color(&fill, -0.4, 0.0), RED);
    assert_eq!(color(&fill, -0.1, 0.0), RED);
    assert_eq!(color(&fill, 0.1, 0.0), BLUE);
    assert_eq!(color(&fill, 0.6, 0.0), CLEAR);
  }

  #[test]
  fn at_its_native_size_each_pixel_is_a_scene_unit() {
    let fill = external(&[], ImageSize::Native);
    // a box 8 by 8 scene units, with the 4 by 2 image in the middle of it
    let at = |x: f32, y: f32| {
      color_in(
        &fill,
        &assets(),
        &Variables::new(),
        Vec2::splat(8.0),
        x / 8.0,
        y / 8.0,
      )
    };
    assert_eq!(at(-1.5, 0.5), RED);
    assert_eq!(at(1.5, -0.5), BLUE);
    assert_eq!(at(3.0, 0.0), CLEAR);
    assert_eq!(at(0.0, 2.0), CLEAR);
  }

  #[test]
  fn its_settings_are_props() {
    let fill = external(
      &[("reds", Prop::Variable("reds".into()))],
      ImageSize::Stretch,
    );
    let mut variables = Variables::new();
    variables.insert("reds".into(), Value::Float(3.0));
    let at = |x: f32| color_in(&fill, &assets(), &variables, Vec2::ONE, x, 0.0);
    assert_eq!(at(0.1), RED);
    assert_eq!(at(0.4), BLUE);
    let mut inputs = Inputs::new();
    fill.read_inputs(&mut inputs);
    assert!(inputs.variables.contains_key("reds"));
  }

  #[test]
  fn a_generator_it_doesnt_have_draws_nothing() {
    let fill = external(&[], ImageSize::Stretch);
    let at = color_in(
      &fill,
      Assets::none(),
      &Variables::new(),
      Vec2::ONE,
      0.0,
      0.0,
    );
    assert_eq!(at, CLEAR);
  }

  #[test]
  fn a_fill_can_only_set_the_settings_its_generator_has() {
    let loaded =
      |fill: Fill| LoadedImageGenerator::for_fill(Arc::new(Columns::default()), &fill.base);
    assert!(
      loaded(external(&[("reds", 1.0.into())], ImageSize::Stretch))
        .is_some_and(|loaded| loaded.is_ok())
    );
    let error = loaded(external(&[("blues", 1.0.into())], ImageSize::Stretch))
      .unwrap()
      .unwrap_err();
    assert_eq!(
      error.to_string(),
      "image generator \"columns\" has no setting \"blues\""
    );
    assert!(loaded(Fill::new(FillNode::image("a.png"))).is_none());
  }

  #[test]
  fn it_round_trips_through_ron_with_nothing_left_at_its_default_written() {
    for (fill, written_with) in [
      (external(&[], ImageSize::Stretch), [false, false]),
      (
        external(&[("reds", 3.0.into())], ImageSize::Native),
        [true, true],
      ),
    ] {
      let written = file::save(&fill).unwrap();
      let read: Fill = file::load(&written).unwrap();
      assert_eq!(
        [written.contains("settings"), written.contains("size")],
        written_with,
        "{written}"
      );
      for x in [-0.4, -0.1, 0.1, 0.4] {
        let at =
          |fill: &Fill| color_in(fill, &assets(), &Variables::new(), Vec2::splat(4.0), x, 0.0);
        assert_eq!(at(&read), at(&fill), "{written}");
      }
    }
  }
}
