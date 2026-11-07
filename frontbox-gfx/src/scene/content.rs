use image::codecs::gif::GifDecoder;
use image::error::{LimitError, LimitErrorKind};
use image::{AnimationDecoder, ImageDecoder, ImageError, ImageFormat, ImageReader, ImageResult, Limits};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::scene::file::{self, FormatError};
use crate::scene::{Images, Picture, Scene};

/// The longest side of an image that will be decoded. A picture bigger than this costs the machine more memory than
/// anything it can show is worth.
const MAX_IMAGE_SIDE: u32 = 8192;

/// The most memory one image's frames can take up, and the most that decoded images are kept in. A small file can
/// decode to far more than a machine has, and it's a machine that has a game to run.
const MAX_DECODED_BYTES: u64 = 512 * 1024 * 1024;

/// The most frames an animated image can have. The cost of a frame goes beyond its pixels, so a file of many tiny ones
/// would get around the limit above.
const MAX_FRAMES: usize = 10_000;

/// The extensions of the files that are images, in lower case
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/// The extension of the files that are scenes, in lower case
const SCENE_EXTENSION: &str = "ron";

/// A folder of what scenes use
///
/// It's scanned whatever is inside it, so it can be organized any way that suits and can hold other files too. A scene
/// names an image by its path from the root with forward slashes, such as `dmd/title.png`, so the name is the same on
/// any machine the folder is on. Scenes are kept in it too, as RON files named the same way.
///
/// Decoded images are kept until their file changes. Clones share them.
#[derive(Debug, Clone)]
pub struct Content {
  root: PathBuf,
  decoded: Arc<Mutex<HashMap<String, Decoded>>>,
}

impl Content {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self { root: root.into(), decoded: Arc::default() }
  }

  pub fn root(&self) -> &Path {
    &self.root
  }

  /// The name of every image in the folder, in order. A folder that isn't there has none.
  pub fn image_names(&self) -> Vec<String> {
    self.names(is_image_name)
  }

  /// The name of every scene in the folder, in order. A folder that isn't there has none.
  pub fn scene_names(&self) -> Vec<String> {
    self.names(is_scene_name)
  }

  fn names(&self, wanted: fn(&str) -> bool) -> Vec<String> {
    let mut names = Vec::new();
    scan(&self.root, "", wanted, &mut names);
    names.sort();
    names
  }

  /// Where the file of a named image is, which doesn't mean it's there
  pub fn image_path(&self, name: &str) -> Result<PathBuf, ContentError> {
    if !is_image_name(name) {
      return Err(ContentError::BadName(name.into()));
    }
    Ok(self.root.join(name))
  }

  /// A decoded image. This reads and decodes the file unless that's already been done, which is too slow for the middle
  /// of a frame.
  pub fn picture(&self, name: &str) -> Result<Arc<Picture>, ContentError> {
    let path = self.image_path(name)?;
    let stamp = fs::metadata(&path).and_then(|metadata| Ok((metadata.modified()?, metadata.len())));
    let stamp = match stamp {
      Ok(stamp) => stamp,
      Err(error) => {
        self.decoded.lock().unwrap().remove(name);
        return Err(ContentError::Read { name: name.into(), error });
      }
    };
    if let Some(decoded) = self.decoded.lock().unwrap().get(name).filter(|decoded| decoded.stamp == stamp) {
      return Ok(decoded.picture.clone());
    }

    let bytes = fs::read(&path).map_err(|error| ContentError::Read { name: name.into(), error })?;
    let picture = Arc::new(decode(name, &bytes)?);
    let decoded = Decoded { stamp, picture: picture.clone() };
    keep(&mut self.decoded.lock().unwrap(), name, decoded, MAX_DECODED_BYTES);
    Ok(picture)
  }

  /// The images a scene draws, to play it with. An image that can't be had is an error here, rather than a layer that's
  /// quietly missing when the scene plays.
  pub fn images_for(&self, scene: &Scene) -> Result<Images, LayerImageError> {
    let mut images = Images::new();
    for (index, layer) in scene.layers.iter().enumerate() {
      if let Some(name) = layer.fill.base.image_name() {
        let picture = self.picture(name).map_err(|error| LayerImageError { layer: layer.label(index), error })?;
        images.insert(name, picture);
      }
    }
    Ok(images)
  }

  /// Adds an image to the folder. It has to decode, and an image that's already there is left as it is, since scenes
  /// could be using it.
  pub fn add_image(&self, name: &str, bytes: &[u8]) -> Result<(), ContentError> {
    let path = self.image_path(name)?;
    decode(name, bytes)?;

    write_new(&path, bytes).map_err(|error| match error.kind() {
      io::ErrorKind::AlreadyExists => ContentError::Taken(name.into()),
      _ => ContentError::Write { name: name.into(), error },
    })
  }

  /// Where the file of a named scene is, which doesn't mean it's there
  pub fn scene_path(&self, name: &str) -> Result<PathBuf, SceneFileError> {
    if !is_scene_name(name) {
      return Err(SceneFileError::BadName(name.into()));
    }
    Ok(self.root.join(name))
  }

  /// A scene's file as it's written. This doesn't check that it's a scene, so that one that isn't can be opened and fixed.
  pub fn read_scene(&self, name: &str) -> Result<String, SceneFileError> {
    let path = self.scene_path(name)?;
    fs::read_to_string(path).map_err(|error| SceneFileError::Read { name: name.into(), error })
  }

  /// Adds a scene to the folder. It has to be one that can be played, and a scene that's already there is left as it is.
  pub fn add_scene(&self, name: &str, source: &str) -> Result<(), SceneFileError> {
    let path = self.scene_path(name)?;
    file::load(source).map_err(|error| SceneFileError::Invalid { name: name.into(), error })?;

    write_new(&path, source.as_bytes()).map_err(|error| match error.kind() {
      io::ErrorKind::AlreadyExists => SceneFileError::Taken(name.into()),
      _ => SceneFileError::Write { name: name.into(), error },
    })
  }

  /// Writes a scene to the folder over any by that name. It has to be one that can be played, and the file is replaced
  /// all at once, so a machine reading it gets one version or the other and a failed save loses nothing.
  pub fn save_scene(&self, name: &str, source: &str) -> Result<(), SceneFileError> {
    let path = self.scene_path(name)?;
    file::load(source).map_err(|error| SceneFileError::Invalid { name: name.into(), error })?;

    let saving = path.with_extension("ron.saving");
    replace(&path, &saving, source.as_bytes()).map_err(|error| {
      let _ = fs::remove_file(&saving);
      SceneFileError::Write { name: name.into(), error }
    })
  }
}

/// A decoded image, and which version of its file it came from
#[derive(Debug)]
struct Decoded {
  stamp: (SystemTime, u64),
  picture: Arc<Picture>,
}

/// Adds to the decoded images, letting go of the others if that makes more than `limit` bytes. Whatever is playing them
/// still has them, and any that are asked for again are decoded again.
fn keep(decoded: &mut HashMap<String, Decoded>, name: &str, added: Decoded, limit: u64) {
  decoded.remove(name);
  let total = decoded.values().map(|decoded| decoded.picture.bytes()).sum::<u64>() + added.picture.bytes();
  if total > limit {
    decoded.clear();
  }
  decoded.insert(name.into(), added);
}

/// Why something in a [`Content`] folder can't be used
#[derive(Debug)]
pub enum ContentError {
  /// Not a path inside the folder with the extension of an image
  BadName(String),
  /// The file couldn't be read, which includes it not being there
  Read { name: String, error: io::Error },
  /// The file isn't an image that can be decoded
  Decode { name: String, error: image::ImageError },
  /// There's already an image by this name
  Taken(String),
  Write { name: String, error: io::Error },
}

impl fmt::Display for ContentError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::BadName(name) => write!(f, "\"{name}\" isn't the name of an image in the content folder"),
      Self::Read { name, error } if error.kind() == io::ErrorKind::NotFound => {
        write!(f, "there's no image \"{name}\" in the content folder")
      }
      Self::Read { name, error } => write!(f, "couldn't read image \"{name}\": {error}"),
      Self::Decode { name, error } => write!(f, "couldn't decode image \"{name}\": {error}"),
      Self::Taken(name) => write!(f, "there's already an image \"{name}\""),
      Self::Write { name, error } => write!(f, "couldn't save image \"{name}\": {error}"),
    }
  }
}

impl std::error::Error for ContentError {}

/// Why a layer of a scene can't have its image
#[derive(Debug)]
pub struct LayerImageError {
  pub layer: String,
  pub error: ContentError,
}

impl fmt::Display for LayerImageError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.layer, self.error)
  }
}

impl std::error::Error for LayerImageError {}

/// Why a scene file in a [`Content`] folder can't be used
#[derive(Debug)]
pub enum SceneFileError {
  /// Not a path inside the folder with the extension of a scene
  BadName(String),
  /// The file couldn't be read, which includes it not being there
  Read { name: String, error: io::Error },
  /// What's being written isn't a scene that can be played
  Invalid { name: String, error: FormatError },
  /// There's already a scene by this name
  Taken(String),
  Write { name: String, error: io::Error },
}

impl fmt::Display for SceneFileError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::BadName(name) => write!(f, "\"{name}\" isn't the name of a scene in the content folder"),
      Self::Read { name, error } if error.kind() == io::ErrorKind::NotFound => {
        write!(f, "there's no scene \"{name}\" in the content folder")
      }
      Self::Read { name, error } => write!(f, "couldn't read scene \"{name}\": {error}"),
      Self::Invalid { name, error } => write!(f, "scene \"{name}\" wasn't saved: {error}"),
      Self::Taken(name) => write!(f, "there's already a scene \"{name}\""),
      Self::Write { name, error } => write!(f, "couldn't save scene \"{name}\": {error}"),
    }
  }
}

impl std::error::Error for SceneFileError {}

/// Symbolic links aren't followed, as one can lead back to a folder this is already in
fn scan(folder: &Path, prefix: &str, wanted: fn(&str) -> bool, names: &mut Vec<String>) {
  let Ok(entries) = fs::read_dir(folder) else {
    return;
  };
  for entry in entries.flatten() {
    let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
      continue;
    };
    let name = format!("{prefix}{file_name}");
    match entry.file_type() {
      Ok(kind) if kind.is_dir() => scan(&entry.path(), &format!("{name}/"), wanted, names),
      Ok(kind) if kind.is_file() && wanted(&name) => names.push(name),
      _ => {}
    }
  }
}

fn is_image_name(name: &str) -> bool {
  is_inside(name) && extension(name).is_some_and(|extension| IMAGE_EXTENSIONS.contains(&extension.as_str()))
}

fn is_scene_name(name: &str) -> bool {
  is_inside(name) && extension(name).is_some_and(|extension| extension == SCENE_EXTENSION)
}

/// A path going down from the root with `/` between its parts, and nothing that can lead out of it on any platform. A
/// scene can be written by anyone who can reach the console, so its names can't be trusted to stay in the folder.
fn is_inside(name: &str) -> bool {
  let safe = |part: &str| !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':']);
  name.split('/').all(safe)
}

fn extension(name: &str) -> Option<String> {
  name.rsplit_once('.').map(|(_, extension)| extension.to_ascii_lowercase())
}

/// Makes a file that has to be new, along with the folders it's in, leaving nothing behind if that fails partway
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
  if let Some(folder) = path.parent() {
    fs::create_dir_all(folder)?;
  }
  let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
  file.write_all(bytes).inspect_err(|_| {
    let _ = fs::remove_file(path);
  })
}

/// Writes `saving` beside a file, then moves it into the file's place
fn replace(path: &Path, saving: &Path, bytes: &[u8]) -> io::Result<()> {
  if let Some(folder) = path.parent() {
    fs::create_dir_all(folder)?;
  }
  fs::write(saving, bytes)?;
  fs::rename(saving, path)
}

fn decode(name: &str, bytes: &[u8]) -> Result<Picture, ContentError> {
  let failed = |error| ContentError::Decode { name: name.into(), error };
  let mut limits = Limits::default();
  limits.max_image_width = Some(MAX_IMAGE_SIDE);
  limits.max_image_height = Some(MAX_IMAGE_SIDE);

  let mut reader = ImageReader::new(Cursor::new(bytes))
    .with_guessed_format()
    .map_err(|error| failed(error.into()))?;
  if reader.format() == Some(ImageFormat::Gif) {
    return gif(bytes, limits).map_err(failed);
  }
  reader.limits(limits);
  reader.decode().map(|image| Picture::new(image.into_rgba8())).map_err(failed)
}

/// Stops at the limits, rather than decoding it all to find out it's too much
fn gif(bytes: &[u8], limits: Limits) -> ImageResult<Picture> {
  let mut decoder = GifDecoder::new(Cursor::new(bytes))?;
  decoder.set_limits(limits)?;

  let (mut frames, mut total, mut duration) = (Vec::new(), 0, 0.0);
  for frame in decoder.into_frames() {
    let frame = frame?;
    let (milliseconds, divisor) = frame.delay().numer_denom_ms();
    duration += milliseconds as f32 / divisor as f32 / 1000.0;

    let image = frame.into_buffer();
    total += image.as_raw().len() as u64;
    if frames.len() == MAX_FRAMES || total > MAX_DECODED_BYTES {
      return Err(ImageError::Limits(LimitError::from_kind(LimitErrorKind::InsufficientMemory)));
    }
    frames.push(image);
  }
  Ok(Picture::animated(frames, duration))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::scene::{Fill, FillNode, Layer, Viewport};
  use image::codecs::gif::GifEncoder;
  use image::{Delay, Rgba, RgbaImage};
  use std::sync::atomic::{AtomicU32, Ordering};

  static NEXT_FOLDER: AtomicU32 = AtomicU32::new(0);

  const RED: Rgba<u8> = Rgba([255, 0, 0, 255]);
  const BLUE: Rgba<u8> = Rgba([0, 0, 255, 255]);

  /// An empty folder of its own, removed afterward
  struct Folder(PathBuf);

  impl Folder {
    fn new() -> Self {
      let id = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
      let path = std::env::temp_dir().join(format!("frontbox-content-{}-{id}", std::process::id()));
      let _ = fs::remove_dir_all(&path);
      fs::create_dir_all(&path).unwrap();
      Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) {
      let path = self.0.join(name);
      fs::create_dir_all(path.parent().unwrap()).unwrap();
      fs::write(path, bytes).unwrap();
    }
  }

  impl Drop for Folder {
    fn drop(&mut self) {
      let _ = fs::remove_dir_all(&self.0);
    }
  }

  fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    RgbaImage::from_pixel(width, height, RED).write_to(&mut bytes, ImageFormat::Png).unwrap();
    bytes.into_inner()
  }

  /// A 2 by 2 frame of each color, 100 milliseconds each
  fn gif_of(colors: impl IntoIterator<Item = Rgba<u8>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    GifEncoder::new(&mut bytes)
      .encode_frames(colors.into_iter().map(|color| {
        let delay = Delay::from_numer_denom_ms(100, 1);
        image::Frame::from_parts(RgbaImage::from_pixel(2, 2, color), 0, 0, delay)
      }))
      .unwrap();
    bytes
  }

  fn drawing(name: &str) -> Scene {
    Scene::new(Viewport::Area, vec![Layer::new(Fill::new(FillNode::image(name)))])
  }

  #[test]
  fn images_are_found_however_the_folder_is_organized() {
    let folder = Folder::new();
    folder.write("title.png", &png(1, 1));
    folder.write("dmd/modes/Hurry Up.PNG", &png(1, 1));
    folder.write("photos/a.jpg", b"not looked at until it's used");
    folder.write("sounds/hit.mp3", b"");
    folder.write("attract.ron", b"");

    assert_eq!(
      Content::new(&folder.0).image_names(),
      ["dmd/modes/Hurry Up.PNG", "photos/a.jpg", "title.png"]
    );
  }

  #[test]
  fn a_folder_that_is_not_there_has_no_images() {
    let content = Content::new("there/is/no/such/folder");
    assert!(content.image_names().is_empty());
    assert!(matches!(content.picture("a.png"), Err(ContentError::Read { .. })));
  }

  #[test]
  fn no_name_leads_out_of_the_folder() {
    let content = Content::new("root");
    for name in [
      "../secret.png",
      "a/../../secret.png",
      "/etc/secret.png",
      "C:/secret.png",
      "a\\..\\secret.png",
      "./a.png",
      "a//b.png",
      "",
    ] {
      assert!(matches!(content.image_path(name), Err(ContentError::BadName(_))), "{name}");
    }
  }

  #[test]
  fn only_images_can_be_named() {
    assert!(matches!(Content::new("root").image_path("attract.ron"), Err(ContentError::BadName(_))));
    assert!(Content::new("root").image_path("dmd/A.PNG").is_ok());
  }

  #[test]
  fn a_scene_gets_the_images_it_names() {
    let folder = Folder::new();
    folder.write("dmd/a.png", &png(3, 2));
    folder.write("unused.png", &png(1, 1));

    let images = Content::new(&folder.0).images_for(&drawing("dmd/a.png")).unwrap();
    assert_eq!(images.get("dmd/a.png").unwrap().width(), 3);
    assert!(images.get("unused.png").is_none());
  }

  #[test]
  fn a_scene_naming_a_missing_image_says_which() {
    let folder = Folder::new();
    let error = Content::new(&folder.0).images_for(&drawing("logo.png")).unwrap_err();
    assert!(error.to_string().contains("logo.png"), "{error}");
  }

  #[test]
  fn a_missing_image_is_blamed_on_the_layer_that_draws_it() {
    let folder = Folder::new();
    let mut scene = drawing("logo.png");
    assert_eq!(Content::new(&folder.0).images_for(&scene).unwrap_err().layer, "layer 1");

    scene.layers[0].name = Some("title".into());
    let error = Content::new(&folder.0).images_for(&scene).unwrap_err();
    assert_eq!(error.to_string(), "layer \"title\": there's no image \"logo.png\" in the content folder");
  }

  #[test]
  fn a_file_that_is_not_an_image_says_which() {
    let folder = Folder::new();
    folder.write("a.png", b"not a png");
    let error = Content::new(&folder.0).picture("a.png").unwrap_err();
    assert!(matches!(error, ContentError::Decode { .. }), "{error}");
  }

  #[test]
  fn an_image_is_decoded_once_until_its_file_changes() {
    let folder = Folder::new();
    folder.write("a.png", &png(2, 2));
    let content = Content::new(&folder.0);

    let first = content.picture("a.png").unwrap();
    assert!(Arc::ptr_eq(&first, &content.picture("a.png").unwrap()));

    // a different size, so the file changes even where the clock is too coarse to see it
    folder.write("a.png", &png(5, 5));
    assert_eq!(content.picture("a.png").unwrap().width(), 5);
  }

  #[test]
  fn an_added_image_can_be_found_and_makes_its_folders() {
    let folder = Folder::new();
    let content = Content::new(folder.0.join("not/made/yet"));
    content.add_image("dmd/a.png", &png(2, 2)).unwrap();
    assert_eq!(content.image_names(), ["dmd/a.png"]);
    assert_eq!(content.picture("dmd/a.png").unwrap().width(), 2);
  }

  #[test]
  fn adding_never_replaces_an_image() {
    let folder = Folder::new();
    folder.write("a.png", &png(2, 2));
    let content = Content::new(&folder.0);

    assert!(matches!(content.add_image("a.png", &png(9, 9)), Err(ContentError::Taken(_))));
    assert_eq!(content.picture("a.png").unwrap().width(), 2);
  }

  #[test]
  fn only_a_real_image_can_be_added() {
    let folder = Folder::new();
    let content = Content::new(&folder.0);
    assert!(matches!(content.add_image("a.png", b"not a png"), Err(ContentError::Decode { .. })));
    assert!(matches!(content.add_image("../a.png", &png(1, 1)), Err(ContentError::BadName(_))));
    assert!(content.image_names().is_empty());
  }

  #[test]
  fn a_gif_keeps_its_frames_and_how_long_they_take() {
    let folder = Folder::new();
    folder.write("a.gif", &gif_of([RED, BLUE, RED]));
    let picture = Content::new(&folder.0).picture("a.gif").unwrap();

    assert_eq!((picture.frames(), picture.width(), picture.height()), (3, 2, 2));
    assert!((picture.duration() - 0.3).abs() < 0.001, "{}", picture.duration());
    assert_eq!(picture.color(1.0, glam::Vec2::ZERO, glam::Vec2::ZERO), BLUE);
  }

  #[test]
  fn a_gif_is_known_by_what_is_in_it_not_its_name() {
    let folder = Folder::new();
    folder.write("a.png", &gif_of([RED, BLUE]));
    assert_eq!(Content::new(&folder.0).picture("a.png").unwrap().frames(), 2);
  }

  #[test]
  fn other_images_have_one_frame() {
    let folder = Folder::new();
    folder.write("a.png", &png(2, 2));
    let picture = Content::new(&folder.0).picture("a.png").unwrap();
    assert_eq!((picture.frames(), picture.duration()), (1, 0.0));
  }

  #[test]
  fn a_gif_with_too_many_frames_is_refused() {
    let folder = Folder::new();
    let content = Content::new(&folder.0);
    let result = content.add_image("a.gif", &gif_of(std::iter::repeat_n(RED, MAX_FRAMES + 1)));
    assert!(matches!(result, Err(ContentError::Decode { .. })));
  }

  #[test]
  fn a_gif_of_tiny_frames_on_a_huge_canvas_is_refused_partway() {
    // each frame costs the whole canvas once decoded, however few bytes it takes in the file
    let frame = |side| {
      let image = RgbaImage::from_pixel(side, side, RED);
      image::Frame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(100, 1))
    };
    let frames = 1 + (MAX_DECODED_BYTES / (2048 * 2048 * 4)) as usize;
    let mut bytes = Vec::new();
    GifEncoder::new(&mut bytes)
      .encode_frames(std::iter::once(frame(2048)).chain((0..frames).map(|_| frame(1))))
      .unwrap();
    assert!(bytes.len() < 1024 * 1024, "{}", bytes.len());

    let folder = Folder::new();
    folder.write("a.gif", &bytes);
    let error = Content::new(&folder.0).picture("a.gif").unwrap_err();
    assert!(matches!(error, ContentError::Decode { .. }), "{error}");
  }

  #[test]
  fn a_gif_that_stops_partway_says_so() {
    let folder = Folder::new();
    let bytes = gif_of([RED, BLUE, RED]);
    folder.write("a.gif", &bytes[..bytes.len() - 8]);
    assert!(matches!(Content::new(&folder.0).picture("a.gif"), Err(ContentError::Decode { .. })));
  }

  #[test]
  fn decoded_images_are_let_go_of_past_the_limit() {
    let decoded = |side| Decoded {
      stamp: (SystemTime::UNIX_EPOCH, 0),
      picture: Arc::new(Picture::new(RgbaImage::new(side, side))),
    };
    let bytes = |side| decoded(side).picture.bytes();
    let limit = bytes(4) + bytes(4);

    let mut kept = HashMap::new();
    keep(&mut kept, "a", decoded(4), limit);
    keep(&mut kept, "b", decoded(4), limit);
    assert_eq!(kept.len(), 2);

    // a new version of one takes that one's place
    keep(&mut kept, "b", decoded(4), limit);
    assert_eq!(kept.len(), 2);

    keep(&mut kept, "c", decoded(4), limit);
    assert_eq!(kept.keys().collect::<Vec<_>>(), ["c"]);
  }

  const SCENE: &str = "(version: 1, viewport: Area, layers: [(fill: (base: Solid(Fixed(255, 0, 0, 255))))])";

  #[test]
  fn scenes_are_found_however_the_folder_is_organized() {
    let folder = Folder::new();
    folder.write("attract.ron", b"");
    folder.write("dmd/modes/Hurry Up.RON", b"");
    folder.write("title.png", &png(1, 1));

    assert_eq!(Content::new(&folder.0).scene_names(), ["attract.ron", "dmd/modes/Hurry Up.RON"]);
  }

  #[test]
  fn no_scene_name_leads_out_of_the_folder() {
    let content = Content::new("root");
    for name in ["../secret.ron", "a/../../secret.ron", "/etc/secret.ron", "C:/secret.ron", "a\\..\\secret.ron", "a.png", ""] {
      assert!(matches!(content.scene_path(name), Err(SceneFileError::BadName(_))), "{name}");
    }
  }

  #[test]
  fn an_added_scene_can_be_read_back_and_makes_its_folders() {
    let folder = Folder::new();
    let content = Content::new(folder.0.join("not/made/yet"));
    content.add_scene("led/a.ron", SCENE).unwrap();
    assert_eq!(content.scene_names(), ["led/a.ron"]);
    assert_eq!(content.read_scene("led/a.ron").unwrap(), SCENE);
  }

  #[test]
  fn adding_never_replaces_a_scene() {
    let folder = Folder::new();
    folder.write("a.ron", b"what was there");
    let content = Content::new(&folder.0);

    assert!(matches!(content.add_scene("a.ron", SCENE), Err(SceneFileError::Taken(_))));
    assert_eq!(content.read_scene("a.ron").unwrap(), "what was there");
  }

  #[test]
  fn saving_replaces_a_scene_and_leaves_nothing_else() {
    let folder = Folder::new();
    folder.write("a.ron", b"what was there");
    let content = Content::new(&folder.0);

    content.save_scene("a.ron", SCENE).unwrap();
    assert_eq!(content.read_scene("a.ron").unwrap(), SCENE);
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 1);
  }

  #[test]
  fn a_scene_that_can_not_be_played_is_not_written() {
    let folder = Folder::new();
    folder.write("a.ron", b"what was there");
    let content = Content::new(&folder.0);

    assert!(matches!(content.save_scene("a.ron", "(oops"), Err(SceneFileError::Invalid { .. })));
    assert!(matches!(content.add_scene("b.ron", "(oops"), Err(SceneFileError::Invalid { .. })));
    assert_eq!(content.read_scene("a.ron").unwrap(), "what was there");
    assert_eq!(content.scene_names(), ["a.ron"]);
  }

  #[test]
  fn a_broken_scene_can_still_be_opened() {
    let folder = Folder::new();
    folder.write("a.ron", b"(oops");
    assert_eq!(Content::new(&folder.0).read_scene("a.ron").unwrap(), "(oops");
  }

  #[test]
  fn a_missing_scene_says_which() {
    let folder = Folder::new();
    let error = Content::new(&folder.0).read_scene("a.ron").unwrap_err();
    assert_eq!(error.to_string(), "there's no scene \"a.ron\" in the content folder");
  }

  #[test]
  fn an_image_too_big_to_be_worth_decoding_is_refused() {
    let folder = Folder::new();
    let content = Content::new(&folder.0);
    let result = content.add_image("a.png", &png(MAX_IMAGE_SIDE + 1, 1));
    assert!(matches!(result, Err(ContentError::Decode { .. })));
  }
}
