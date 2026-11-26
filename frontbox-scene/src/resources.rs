//! What a scene is made from, as each is kept in the app's [`Resources`]

use std::fmt;
use std::sync::Arc;

use frontbox_core::prelude::*;
use frontbox_gfx::scene::{
  Analysis, Assets, EnvelopeContent, FONT_EXTENSIONS, Font, GeneratorError, IMAGE_EXTENSIONS,
  Layer, LayerGeneratorError, LoadedGenerator, Picture, SOUND_EXTENSIONS, ShapeGenerator, file,
};

/// Scenes, as each one's source: its `.scene` file's text, not parsed, so a broken file can still be read and fixed.
/// [`Scene::open`](crate::Scene::open) reads one to play.
///
/// A crate adds the scenes it comes with by the path a game's own file replaces each under:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// # use frontbox_scene::*;
/// # let mut resources = Resources::without_folder();
/// resources.add::<Scenes>("frontbox/points.scene", "Scene(version: 2)".into());
/// assert!(Scene::open(&resources, "frontbox/points.scene").is_ok());
/// ```
pub struct Scenes;

impl Resource for Scenes {
  type Item = String;
  const NAME: &'static str = "scene";
  const EXTENSIONS: &'static [&'static str] = &[file::EXTENSION];

  fn load(_name: &str, bytes: Vec<u8>) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|_| "it isn't text".into())
  }
}

/// Images a scene draws, decoded. Decoded images are let go of once they pass [`Picture::MAX_DECODED_BYTES`] between
/// them, and decoded again when next asked for.
pub struct Images;

impl Resource for Images {
  type Item = Arc<Picture>;
  const NAME: &'static str = "image";
  const EXTENSIONS: &'static [&'static str] = &IMAGE_EXTENSIONS;
  const KEPT_SIZE: u64 = Picture::MAX_DECODED_BYTES;

  fn load(_name: &str, bytes: Vec<u8>) -> Result<Arc<Picture>, String> {
    Picture::decode(&bytes)
      .map(Arc::new)
      .map_err(|error| error.to_string())
  }

  fn size(picture: &Arc<Picture>) -> u64 {
    picture.bytes()
  }
}

/// Fonts a scene's text is in
pub struct Fonts;

impl Resource for Fonts {
  type Item = Font;
  const NAME: &'static str = "font";
  const EXTENSIONS: &'static [&'static str] = &FONT_EXTENSIONS;

  fn load(_name: &str, bytes: Vec<u8>) -> Result<Font, String> {
    Font::new(bytes).map_err(|_| "it isn't a TrueType or OpenType font".into())
  }
}

/// Sounds a scene's sound layers play. Their files are only found ([`Resources::path`]); whatever plays them
/// loads them.
pub struct Sounds;

impl Resource for Sounds {
  type Item = ();
  const NAME: &'static str = "sound";
  const EXTENSIONS: &'static [&'static str] = &SOUND_EXTENSIONS;
}

/// Shape generators, each by the name a scene's `External` shape gives it. They're the app's code, so they're only
/// ever added:
///
/// ```rust
/// # use std::sync::Arc;
/// # use frontbox_core::prelude::*;
/// # use frontbox_gfx::scene::*;
/// # use frontbox_scene::*;
/// #[derive(Debug)]
/// struct Tentacle;
///
/// impl ShapeGenerator for Tentacle {
///   fn settings(&self) -> &[GeneratorSetting] {
///     &[]
///   }
///
///   fn generate(&self, _frame: &GeneratorFrame) -> Generated {
///     Generated::empty()
///   }
/// }
///
/// # let mut app = App::new(BootConfig { config_path: None, ..Default::default() });
/// app.resources().add::<ShapeGenerators>("tentacle", Arc::new(Tentacle));
/// ```
pub struct ShapeGenerators;

impl Resource for ShapeGenerators {
  type Item = Arc<dyn ShapeGenerator>;
  const NAME: &'static str = "shape generator";
}

/// The images, fonts, and shape generators `layers` draw, from `resources`. Each layer comes with its index among the
/// scene's layers, to name it by in errors. A missing one is an error here, rather than a layer silently drawing
/// nothing, as is a setting its generator doesn't have.
pub(crate) fn assets_for<'a>(
  resources: &Resources,
  layers: impl IntoIterator<Item = (usize, &'a Layer)>,
) -> Result<Assets, LayerAssetError> {
  let mut assets = Assets::new();
  for (index, layer) in layers {
    let failed = |error: AssetError| LayerAssetError {
      layer: layer.label(index),
      error,
    };
    for name in layer.image_names() {
      let picture = resources.get::<Images>(name);
      assets.insert_picture(name, picture.map_err(|error| failed(error.into()))?);
    }
    for name in layer.font_names() {
      let font = resources.get::<Fonts>(name);
      assets.insert_font(name, font.map_err(|error| failed(error.into()))?);
    }
    if let Some(shape) = &layer.region
      && let Some(name) = shape.base.generator_name()
    {
      let generator = resources
        .get::<ShapeGenerators>(name)
        .map_err(|error| failed(error.into()))?;
      if let Some(loaded) = LoadedGenerator::for_shape(generator, &shape.base) {
        assets.insert_shape_generator(name, loaded.map_err(|error| failed(error.into()))?);
      }
    }
  }
  Ok(assets)
}

/// Sound `name` analyzed by `analysis`, for an external envelope to follow. Decodes the whole sound the first time, so
/// don't call this mid-frame.
pub(crate) fn envelope(
  resources: &Resources,
  name: &str,
  analysis: Analysis,
) -> Result<Arc<EnvelopeContent>, ResourceError> {
  match analysis {
    Analysis::Rms => resources.get::<RmsEnvelopes>(name),
  }
}

/// Sounds, each analyzed by [`Analysis::Rms`]
struct RmsEnvelopes;

impl Resource for RmsEnvelopes {
  type Item = Arc<EnvelopeContent>;
  const NAME: &'static str = "sound";
  const EXTENSIONS: &'static [&'static str] = &SOUND_EXTENSIONS;

  fn load(name: &str, bytes: Vec<u8>) -> Result<Arc<EnvelopeContent>, String> {
    EnvelopeContent::analyze(name, bytes, Analysis::Rms).map(Arc::new)
  }
}

/// Why a scene can't have something it draws with
#[derive(Debug)]
pub enum AssetError {
  /// It couldn't be had from the app's resources
  Resources(ResourceError),
  /// Its shape generator can't be used as the scene does
  Generator(GeneratorError),
}

impl fmt::Display for AssetError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Resources(error) => error.fmt(f),
      Self::Generator(error) => error.fmt(f),
    }
  }
}

impl std::error::Error for AssetError {}

impl From<ResourceError> for AssetError {
  fn from(error: ResourceError) -> Self {
    Self::Resources(error)
  }
}

impl From<GeneratorError> for AssetError {
  fn from(error: GeneratorError) -> Self {
    Self::Generator(error)
  }
}

/// A layer can't have something it draws with
#[derive(Debug)]
pub struct LayerAssetError {
  pub layer: String,
  pub error: AssetError,
}

impl fmt::Display for LayerAssetError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.layer, self.error)
  }
}

impl std::error::Error for LayerAssetError {}

impl From<LayerGeneratorError> for LayerAssetError {
  fn from(error: LayerGeneratorError) -> Self {
    Self {
      layer: error.layer,
      error: error.error.into(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_folder::Folder;
  use frontbox_gfx::scene::{
    Fill, FillNode, Generated, GeneratorFrame, GeneratorSetting, Shape, ShapeNode,
  };
  use std::collections::BTreeMap;

  const PIXEL_FONT: &[u8] = include_bytes!("../../frontbox-gfx/tests/fonts/LCDBlock.ttf");

  fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(Vec::new());
    image::RgbaImage::new(width, height)
      .write_to(&mut bytes, image::ImageFormat::Png)
      .unwrap();
    bytes.into_inner()
  }

  fn drawing(name: &str) -> Layer {
    Layer::new(Fill::new(FillNode::image(name)))
  }

  fn writing(font: &str) -> Layer {
    let shape = Shape::new(ShapeNode::text("HI", font, 8.0, Vec2::new(16.0, 8.0)));
    drawing("a.png").region(shape)
  }

  fn load(resources: &Resources, layer: &Layer) -> Result<Assets, LayerAssetError> {
    assets_for(resources, [(0, layer)])
  }

  #[test]
  fn each_kind_is_found_by_its_extensions() {
    let folder = Folder::new();
    folder.write("title.png", b"not looked at until it's used");
    folder.write("dmd/modes/Hurry Up.JPG", b"");
    folder.write("fonts/pixel.ttf", b"");
    folder.write("fonts/Other.OTF", b"");
    folder.write("sounds/hit.mp3", b"");
    folder.write("sounds/callouts/Jackpot.WAV", b"");
    folder.write("attract.scene", b"");
    folder.write("notes.txt", b"");
    let resources = Resources::new(&folder.0);

    assert_eq!(
      resources.names::<Images>(),
      ["dmd/modes/Hurry Up.JPG", "title.png"]
    );
    assert_eq!(
      resources.names::<Fonts>(),
      ["fonts/Other.OTF", "fonts/pixel.ttf"]
    );
    assert_eq!(
      resources.names::<Sounds>(),
      ["sounds/callouts/Jackpot.WAV", "sounds/hit.mp3"]
    );
    assert_eq!(resources.names::<Scenes>(), ["attract.scene"]);
    assert!(resources.path::<Sounds>("sounds/hit.mp3").is_ok());
    assert!(matches!(
      resources.path::<Sounds>("title.png"),
      Err(ResourceError::BadName { .. })
    ));
  }

  #[test]
  fn a_scene_gets_the_images_it_names() {
    let folder = Folder::new();
    folder.write("dmd/a.png", &png(3, 2));
    folder.write("unused.png", &png(1, 1));

    let assets = load(&Resources::new(&folder.0), &drawing("dmd/a.png")).unwrap();
    assert_eq!(assets.picture("dmd/a.png").unwrap().width(), 3);
    assert!(assets.picture("unused.png").is_none());
  }

  #[test]
  fn a_missing_image_is_blamed_on_the_layer_that_draws_it() {
    let folder = Folder::new();
    let resources = Resources::new(&folder.0);
    let mut layer = drawing("logo.png");
    assert_eq!(load(&resources, &layer).unwrap_err().layer, "layer 1");

    layer.name = Some("title".into());
    assert_eq!(
      load(&resources, &layer).unwrap_err().to_string(),
      "layer \"title\": there's no image \"logo.png\" in the content folder"
    );
  }

  #[test]
  fn a_file_that_is_not_an_image_or_a_font_says_which() {
    let folder = Folder::new();
    folder.write("a.png", b"not a png");
    folder.write("a.ttf", b"not a font");
    let resources = Resources::new(&folder.0);
    let error = resources.get::<Images>("a.png").unwrap_err().to_string();
    assert!(
      error.starts_with("couldn't load image \"a.png\": "),
      "{error}"
    );
    assert_eq!(
      resources.get::<Fonts>("a.ttf").unwrap_err().to_string(),
      "couldn't load font \"a.ttf\": it isn't a TrueType or OpenType font"
    );
  }

  #[test]
  fn a_scene_gets_the_fonts_its_text_is_in() {
    let folder = Folder::new();
    folder.write("a.png", &png(1, 1));
    folder.write("fonts/pixel.ttf", PIXEL_FONT);
    let resources = Resources::new(&folder.0);

    let assets = load(&resources, &writing("fonts/pixel.ttf")).unwrap();
    assert!(assets.font("fonts/pixel.ttf").is_some());
    assert!(assets.picture("a.png").is_some());
    assert_eq!(
      load(&resources, &writing("fonts/other.ttf"))
        .unwrap_err()
        .to_string(),
      "layer 1: there's no font \"fonts/other.ttf\" in the content folder"
    );
  }

  #[test]
  fn a_font_a_crate_comes_with_is_used_until_a_file_replaces_it() {
    let folder = Folder::new();
    folder.write("a.png", &png(1, 1));
    let mut resources = Resources::new(&folder.0);
    let font = Font::new(PIXEL_FONT.to_vec()).unwrap();
    resources.add::<Fonts>("frontbox/fonts/Pixel.ttf", font);

    assert!(load(&resources, &writing("frontbox/fonts/Pixel.ttf")).is_ok());
    assert_eq!(resources.names::<Fonts>(), ["frontbox/fonts/Pixel.ttf"]);

    folder.write("frontbox/fonts/Pixel.ttf", b"not a font");
    let error = load(&resources, &writing("frontbox/fonts/Pixel.ttf")).unwrap_err();
    assert!(
      matches!(
        error.error,
        AssetError::Resources(ResourceError::Load { .. })
      ),
      "{error}"
    );
  }

  /// A unit square, `size` across
  #[derive(Debug)]
  struct Square;

  impl ShapeGenerator for Square {
    fn settings(&self) -> &[GeneratorSetting] {
      const SETTINGS: &[GeneratorSetting] = &[GeneratorSetting::new("size", 1.0)];
      SETTINGS
    }

    fn generate(&self, frame: &GeneratorFrame) -> Generated {
      let half = frame.setting("size") / 2.0;
      let corners = [(-half, -half), (half, -half), (half, half), (-half, half)];
      Generated::areas([corners.map(|(x, y)| Vec2::new(x, y)).to_vec()])
    }
  }

  #[test]
  fn a_layer_gets_the_generator_it_names_and_only_the_settings_it_has() {
    let mut resources = Resources::without_folder();
    resources.add::<ShapeGenerators>("square", Arc::new(Square));
    let external = |generator: &str, setting: &str| {
      let settings = BTreeMap::from([(setting.to_owned(), 1.0.into())]);
      let shape = Shape::new(ShapeNode::External {
        generator: generator.into(),
        settings,
      });
      drawing("").region(shape)
    };

    let assets = load(&resources, &external("square", "size")).unwrap();
    assert!(assets.shape_generator("square").is_some());
    assert_eq!(
      load(&resources, &external("tentacle", "size"))
        .unwrap_err()
        .to_string(),
      "layer 1: the app has no shape generator \"tentacle\""
    );
    assert_eq!(
      load(&resources, &external("square", "curl"))
        .unwrap_err()
        .to_string(),
      "layer 1: shape generator \"square\" has no setting \"curl\""
    );
  }

  #[test]
  fn a_scene_opened_while_the_app_is_built_finds_what_was_added_before_it() {
    let mut app = App::new(BootConfig {
      config_path: None,
      content_path: None,
      ..Default::default()
    });
    app.resources().add::<Scenes>(
      "squares.scene",
      r#"Scene(version: 2, layers: [Display(display: "Dmd", layer: (
        fill: (base: Solid(Fixed(255, 0, 0, 255))),
        region: (base: External(generator: "square", settings: {"size": Fixed(2.0)})),
      ))])"#
        .into(),
    );
    assert_eq!(
      crate::Scene::open(app.resources(), "squares.scene")
        .unwrap_err()
        .to_string(),
      "scene \"squares.scene\": layer 1: the app has no shape generator \"square\""
    );

    app
      .resources()
      .add::<ShapeGenerators>("square", Arc::new(Square));
    let scene = crate::Scene::open(app.resources(), "squares.scene").unwrap();
    assert!(scene.assets.shape_generator("square").is_some());
  }
}
