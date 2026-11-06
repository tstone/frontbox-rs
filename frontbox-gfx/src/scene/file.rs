//! Reading and writing scenes as RON
//!
//! Files are written with RON's `implicit_some` and `unwrap_variant_newtypes` extensions, so they only state what
//! they change and avoid double parentheses. Fields that aren't written take their default.

use std::fmt;

use ron::extensions::Extensions;
use ron::ser::PrettyConfig;
use serde::Deserialize;

use crate::scene::Scene;

/// The format version this build writes, and the newest it reads
pub const VERSION: u32 = 1;

/// Reads a scene from RON
pub fn load(source: &str) -> Result<Scene, FormatError> {
  // read the version on its own first, so a newer file is reported as newer rather than as a parse error from a
  // variant this build doesn't know
  #[derive(Deserialize)]
  struct Header {
    version: u32,
  }
  let header: Header = options().from_str(source).map_err(FormatError::Read)?;
  if header.version > VERSION {
    return Err(FormatError::Newer { version: header.version });
  }

  options().from_str(source).map_err(FormatError::Read)
}

/// Writes a scene as RON
pub fn save(scene: &Scene) -> Result<String, FormatError> {
  let config = PrettyConfig::new().indentor("  ").extensions(extensions());
  options().to_string_pretty(scene, config).map_err(FormatError::Write)
}

fn extensions() -> Extensions {
  Extensions::IMPLICIT_SOME | Extensions::UNWRAP_VARIANT_NEWTYPES
}

fn options() -> ron::Options {
  ron::Options::default().with_default_extension(extensions())
}

#[derive(Debug)]
pub enum FormatError {
  Read(ron::error::SpannedError),
  Write(ron::Error),
  /// The file was written by a newer version of the format
  Newer { version: u32 },
}

impl fmt::Display for FormatError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Read(e) => write!(f, "couldn't read scene: {e}"),
      Self::Write(e) => write!(f, "couldn't write scene: {e}"),
      Self::Newer { version } => {
        write!(f, "scene is format version {version}, but this build reads up to {VERSION}")
      }
    }
  }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
mod tests {
  use glam::Vec2;
  use image::Rgba;

  use super::*;
  use crate::animation::{Curve, Cycle};
  use crate::layout::Line;
  use crate::scene::*;

  fn sample_grid(scene: &Scene) -> Vec<Rgba<u8>> {
    let params = Params::new();
    [0.0, 0.5, 1.0, 1.5]
      .into_iter()
      .flat_map(|time| {
        let frame = Frame { time, params: &params, size: Vec2::new(100.0, 50.0) };
        (0..20)
          .flat_map(|y| (0..40).map(move |x| Sample::new(Vec2::new(x as f32 * 2.5, y as f32 * 2.5), 1.0)))
          .map(move |s| scene.sample(&s, &frame))
      })
      .collect()
  }

  /// Uses every type in a definition
  fn everything() -> Scene {
    let mut scene = Scene::new(Viewport::Area, vec![
        Layer::new(Paint::new(PaintNode::Solid(Prop::Param("background".into())))),
        Layer::new(
          Paint::new(PaintNode::Gradient {
            stops: vec![
              Stop::new(0.0, Color(Rgba([255, 204, 0, 255]))),
              Stop::new(1.0, Color(Rgba([255, 51, 0, 128]))),
            ],
            space: ColorSpace::Oklab,
          })
          .warp(Warp::Rotate(90.0.into()))
          .adjust(ColorAdjust::HueShift(Prop::Param("hue".into()))),
        )
        .shape(
          Shape::new(ShapeNode::Ellipse)
            .warp(Warp::Scale(Vec2::splat(40.0).into()))
            .warp(Warp::Translate(Prop::Track(Track {
              keys: vec![
                Key::new(0.0, Vec2::new(30.0, 25.0), Curve::EaseInOut),
                Key::new(1.0, Vec2::new(70.0, 25.0), Curve::Linear),
              ],
              cycle: Cycle::Forever,
            }))),
        ),
        Layer::new(Paint::new(PaintNode::Solid(Color(Rgba([0, 0, 255, 255])).into()))).shape(Shape::new(
          ShapeNode::Polygon {
            points: vec![Vec2::new(0.0, 0.0).into(), Vec2::new(20.0, 0.0).into(), Vec2::new(10.0, 15.0).into()],
          },
        )),
    ]);
    scene.params.insert("background".into(), Value::Color(Color(Rgba([16, 16, 16, 255]))));
    scene
  }

  #[test]
  fn round_trip_samples_the_same() {
    let scene = everything();
    let written = save(&scene).unwrap();
    let read = load(&written).unwrap();
    assert_eq!(sample_grid(&read), sample_grid(&scene), "written as:\n{written}");
    assert_eq!(save(&read).unwrap(), written);
  }

  #[test]
  fn reads_a_hand_written_file() {
    let scene = load(
      r#"(
        version: 1,
        viewport: Sequence,
        layers: [
          (paint: (base: Gradient(stops: [
            (at: 0.0, color: Fixed(255, 0, 0, 255)),
            (at: 1.0, color: Fixed(0, 0, 255, 255)),
          ]))),
        ],
      )"#,
    )
    .unwrap();

    let params = Params::new();
    let line = Line::new(3);
    let frame = Frame { time: 0.0, params: &params, size: Vec2::new(100.0, 50.0) };
    let colors: Vec<_> = line.samples().map(|s| scene.sample(&s, &frame)).collect();
    assert_eq!(colors[0], Rgba([255, 0, 0, 255]));
    assert_eq!(colors[2], Rgba([0, 0, 255, 255]));
  }

  #[test]
  fn reads_an_area_viewport() {
    let scene = load("(version: 1, viewport: Area, layers: [])").unwrap();
    assert_eq!(scene.viewport, Viewport::Area);
  }

  #[test]
  fn a_scene_must_state_its_viewport() {
    assert!(matches!(load("(version: 1, layers: [])"), Err(FormatError::Read(_))));
  }

  #[test]
  fn rejects_a_newer_version() {
    let result = load("(version: 2, layers: [(paint: (base: SomethingNew))])");
    assert!(matches!(result, Err(FormatError::Newer { version: 2 })), "{result:?}");
  }
}
