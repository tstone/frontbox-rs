use image::codecs::gif::GifDecoder;
use image::error::{LimitError, LimitErrorKind};
use image::{AnimationDecoder, ImageDecoder, ImageError, ImageFormat, ImageReader, ImageResult, Limits};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::scene::file;
use crate::scene::{Assets, Font, Layer, Picture};

/// Largest width or height that will be decoded
const MAX_IMAGE_SIDE: u32 = 8192;

/// Memory cap for one image's decoded frames, and for the whole decoded image cache. Guards against small files that
/// decode to huge sizes.
const MAX_DECODED_BYTES: u64 = 512 * 1024 * 1024;

/// LayerFrame cap for animated images. Each frame has a cost beyond its pixels, so many tiny frames could otherwise get
/// around the byte cap.
const MAX_FRAMES: usize = 10_000;

/// Image file extensions, in lowercase
const IMAGE_EXTENSIONS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "webp", "bmp"];

/// What a [`ContentError::NoFolder`] or [`SceneFileError::NoFolder`] says
const NO_FOLDER: &str = "the app has no content folder";

/// Font file extensions, in lowercase
const FONT_EXTENSIONS: [&str; 2] = ["ttf", "otf"];

/// Sound file extensions, in lowercase: the formats the sound system decodes
const SOUND_EXTENSIONS: [&str; 4] = ["wav", "mp3", "ogg", "flac"];

/// The content folder, holding scenes and the images, fonts, and sounds they use
///
/// The folder is scanned recursively, so it can be organized freely and hold other files too. Images, fonts, sounds,
/// and scenes are named by their path from the root with forward slashes, such as `dmd/title.png`, so names are the
/// same on every OS.
///
/// Fonts can also come from [`Self::with_fonts`], such as those a crate comes with, named without an extension so they
/// can't be mistaken for a file.
///
/// An app without a folder still has a `Content`, from [`Self::without_folder`]. It has only the fonts it's given, and
/// anything that needs a file is a [`ContentError::NoFolder`] or [`SceneFileError::NoFolder`].
///
/// Decoded images and loaded fonts are cached until their file changes. Clones share the cache.
#[derive(Debug, Clone)]
pub struct Content {
  /// `None` for an app without a folder
  root: Option<PathBuf>,
  decoded: Arc<Mutex<HashMap<String, Decoded>>>,
  /// Loaded fonts, with their file's modified time and size when they were loaded
  fonts: Arc<Mutex<HashMap<String, (Stamp, Font)>>>,
  /// From [`Self::with_fonts`], by name
  bundled: Arc<BTreeMap<String, Font>>,
}

impl Content {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self { root: Some(root.into()), ..Self::without_folder() }
  }

  /// For an app without a content folder
  pub fn without_folder() -> Self {
    Self { root: None, decoded: Arc::default(), fonts: Arc::default(), bundled: Arc::default() }
  }

  /// Adds fonts that aren't in the folder, such as those a crate comes with. A name given here is used before a file
  /// of the same name, so give them names without an extension, which a file in the folder can't have.
  pub fn with_fonts<'a>(mut self, fonts: impl IntoIterator<Item = (&'a str, Font)>) -> Self {
    let mut bundled = (*self.bundled).clone();
    bundled.extend(fonts.into_iter().map(|(name, font)| (name.to_owned(), font)));
    self.bundled = Arc::new(bundled);
    self
  }

  /// The folder. `None` for an app without one.
  pub fn root(&self) -> Option<&Path> {
    self.root.as_deref()
  }

  /// Every image name, sorted. Empty if the folder doesn't exist.
  pub fn image_names(&self) -> Vec<String> {
    self.names(is_image_name)
  }

  /// Every font name, sorted: those from [`Self::with_fonts`], and those in the folder
  pub fn font_names(&self) -> Vec<String> {
    let mut names: Vec<String> = self.bundled.keys().cloned().chain(self.names(is_font_name)).collect();
    names.sort();
    names
  }

  /// Every sound name, sorted. Empty if the folder doesn't exist.
  pub fn sound_names(&self) -> Vec<String> {
    self.names(is_sound_name)
  }

  /// Every scene name, sorted. Empty if the folder doesn't exist.
  pub fn scene_names(&self) -> Vec<String> {
    self.names(is_scene_name)
  }

  fn names(&self, wanted: fn(&str) -> bool) -> Vec<String> {
    let mut names = Vec::new();
    if let Some(root) = &self.root {
      scan(root, "", wanted, &mut names);
    }
    names.sort();
    names
  }

  /// The file path for an image name. Doesn't check that the file exists.
  pub fn image_path(&self, name: &str) -> Result<PathBuf, ContentError> {
    if !is_image_name(name) {
      return Err(ContentError::BadName(name.into()));
    }
    self.file(name).ok_or(ContentError::NoFolder)
  }

  /// The file path for a sound name. Doesn't check that the file exists.
  pub fn sound_path(&self, name: &str) -> Result<PathBuf, ContentError> {
    if !is_sound_name(name) {
      return Err(ContentError::BadName(name.into()));
    }
    self.file(name).ok_or(ContentError::NoFolder)
  }

  /// Where a file by this name would be in the folder. `None` without a folder.
  fn file(&self, name: &str) -> Option<PathBuf> {
    self.root.as_ref().map(|root| root.join(name))
  }

  /// A decoded image, from the cache unless its file has changed. Decoding is slow, so don't call this mid-frame.
  pub fn picture(&self, name: &str) -> Result<Arc<Picture>, ContentError> {
    let path = self.image_path(name)?;
    let stamp = match stamp(&path) {
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

  /// A font from [`Self::with_fonts`], or one loaded from the folder, from the cache unless its file has changed.
  /// Reads the file, so don't call this mid-frame.
  pub fn font(&self, name: &str) -> Result<Font, ContentError> {
    if let Some(font) = self.bundled.get(name) {
      return Ok(font.clone());
    }
    if !is_font_name(name) {
      return Err(ContentError::BadFontName(name.into()));
    }
    let path = self.file(name).ok_or(ContentError::NoFolder)?;
    let read = |error| ContentError::ReadFont { name: name.into(), error };
    let stamp = match stamp(&path) {
      Ok(stamp) => stamp,
      Err(error) => {
        self.fonts.lock().unwrap().remove(name);
        return Err(read(error));
      }
    };
    if let Some((_, font)) = self.fonts.lock().unwrap().get(name).filter(|(loaded, _)| *loaded == stamp) {
      return Ok(font.clone());
    }
    let font = Font::new(fs::read(&path).map_err(read)?).map_err(|_| ContentError::NotAFont(name.into()))?;
    self.fonts.lock().unwrap().insert(name.into(), (stamp, font.clone()));
    Ok(font)
  }

  /// Loads every image and font a scene's layers use, given in the scene's order. A missing one is an error here,
  /// rather than a layer silently drawing nothing.
  pub fn assets_for<'a>(&self, layers: impl IntoIterator<Item = &'a Layer>) -> Result<Assets, LayerAssetError> {
    let mut assets = Assets::new();
    for (index, layer) in layers.into_iter().enumerate() {
      let failed = |error| LayerAssetError { layer: layer.label(index), error };
      for name in layer.image_names() {
        assets.insert_picture(name, self.picture(name).map_err(failed)?);
      }
      for name in layer.font_names() {
        assets.insert_font(name, self.font(name).map_err(failed)?);
      }
    }
    Ok(assets)
  }

  /// Adds a new image. It must decode, and an existing image is never overwritten, since scenes may be using it.
  pub fn add_image(&self, name: &str, bytes: &[u8]) -> Result<(), ContentError> {
    let path = self.image_path(name)?;
    decode(name, bytes)?;

    write_new(&path, bytes).map_err(|error| match error.kind() {
      io::ErrorKind::AlreadyExists => ContentError::Taken(name.into()),
      _ => ContentError::Write { name: name.into(), error },
    })
  }

  /// The file path for a scene name. Doesn't check that the file exists.
  pub fn scene_path(&self, name: &str) -> Result<PathBuf, SceneFileError> {
    if !is_scene_name(name) {
      return Err(SceneFileError::BadName(name.into()));
    }
    self.file(name).ok_or(SceneFileError::NoFolder)
  }

  /// A scene file's raw text. It isn't parsed, so a broken file can still be opened and fixed.
  pub fn read_scene(&self, name: &str) -> Result<String, SceneFileError> {
    let path = self.scene_path(name)?;
    fs::read_to_string(path).map_err(|error| SceneFileError::Read { name: name.into(), error })
  }

  /// Adds a new scene. Fails if one by that name exists.
  pub fn add_scene(&self, name: &str, source: &str) -> Result<(), SceneFileError> {
    let path = self.scene_path(name)?;

    write_new(&path, source.as_bytes()).map_err(|error| match error.kind() {
      io::ErrorKind::AlreadyExists => SceneFileError::Taken(name.into()),
      _ => SceneFileError::Write { name: name.into(), error },
    })
  }

  /// Writes a scene, replacing any by that name. It writes a temp file and renames it into place, so readers never see a
  /// partial file and a failed save loses nothing.
  pub fn save_scene(&self, name: &str, source: &str) -> Result<(), SceneFileError> {
    let path = self.scene_path(name)?;

    let saving = path.with_extension("scene.saving");
    replace(&path, &saving, source.as_bytes()).map_err(|error| {
      let _ = fs::remove_file(&saving);
      SceneFileError::Write { name: name.into(), error }
    })
  }

  /// Deletes a scene file. Its parent folders are left, even if empty.
  pub fn delete_scene(&self, name: &str) -> Result<(), SceneFileError> {
    let path = self.scene_path(name)?;
    fs::remove_file(path).map_err(|error| SceneFileError::Delete { name: name.into(), error })
  }
}

/// A cached decoded image
#[derive(Debug)]
struct Decoded {
  /// The file's modified time and size when it was decoded
  stamp: Stamp,
  picture: Arc<Picture>,
}

/// A file's modified time and size, which change when it's written
type Stamp = (SystemTime, u64);

fn stamp(path: &Path) -> io::Result<Stamp> {
  let metadata = fs::metadata(path)?;
  Ok((metadata.modified()?, metadata.len()))
}

/// Adds to the cache, clearing everything else if the total would exceed `limit` bytes. Cleared images stay alive for
/// anything still holding them.
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
  /// The app has no content folder, so there's no file to use
  NoFolder,
  /// Not a valid image path inside the folder
  BadName(String),
  /// Couldn't read the file, including when it doesn't exist
  Read { name: String, error: io::Error },
  Decode { name: String, error: image::ImageError },
  /// Not a valid font path inside the folder
  BadFontName(String),
  /// Couldn't read a font's file, including when it doesn't exist
  ReadFont { name: String, error: io::Error },
  NotAFont(String),
  /// An image by this name already exists
  Taken(String),
  Write { name: String, error: io::Error },
}

impl fmt::Display for ContentError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoFolder => f.write_str(NO_FOLDER),
      Self::BadName(name) => write!(f, "\"{name}\" isn't the name of an image in the content folder"),
      Self::Read { name, error } if error.kind() == io::ErrorKind::NotFound => {
        write!(f, "there's no image \"{name}\" in the content folder")
      }
      Self::Read { name, error } => write!(f, "couldn't read image \"{name}\": {error}"),
      Self::Decode { name, error } => write!(f, "couldn't decode image \"{name}\": {error}"),
      Self::BadFontName(name) => write!(f, "\"{name}\" isn't the name of a font in the content folder"),
      Self::ReadFont { name, error } if error.kind() == io::ErrorKind::NotFound => {
        write!(f, "there's no font \"{name}\" in the content folder")
      }
      Self::ReadFont { name, error } => write!(f, "couldn't read font \"{name}\": {error}"),
      Self::NotAFont(name) => write!(f, "\"{name}\" isn't a TrueType or OpenType font"),
      Self::Taken(name) => write!(f, "there's already an image \"{name}\""),
      Self::Write { name, error } => write!(f, "couldn't save image \"{name}\": {error}"),
    }
  }
}

impl std::error::Error for ContentError {}

/// A layer's image or font failed to load
#[derive(Debug)]
pub struct LayerAssetError {
  pub layer: String,
  pub error: ContentError,
}

impl fmt::Display for LayerAssetError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}: {}", self.layer, self.error)
  }
}

impl std::error::Error for LayerAssetError {}

/// Why a scene file in a [`Content`] folder can't be used
#[derive(Debug)]
pub enum SceneFileError {
  /// The app has no content folder, so there's no file to use
  NoFolder,
  /// Not a valid scene path inside the folder
  BadName(String),
  /// Couldn't read the file, including when it doesn't exist
  Read { name: String, error: io::Error },
  /// A scene by this name already exists
  Taken(String),
  Write { name: String, error: io::Error },
  /// Couldn't delete the file, including when it doesn't exist
  Delete { name: String, error: io::Error },
}

impl fmt::Display for SceneFileError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::NoFolder => f.write_str(NO_FOLDER),
      Self::BadName(name) => write!(f, "\"{name}\" isn't the name of a scene in the content folder"),
      Self::Read { name, error } | Self::Delete { name, error } if error.kind() == io::ErrorKind::NotFound => {
        write!(f, "there's no scene \"{name}\" in the content folder")
      }
      Self::Read { name, error } => write!(f, "couldn't read scene \"{name}\": {error}"),
      Self::Taken(name) => write!(f, "there's already a scene \"{name}\""),
      Self::Write { name, error } => write!(f, "couldn't save scene \"{name}\": {error}"),
      Self::Delete { name, error } => write!(f, "couldn't delete scene \"{name}\": {error}"),
    }
  }
}

impl std::error::Error for SceneFileError {}

/// Recursively collects the names of matching files. Symlinks aren't followed, to avoid cycles.
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

fn is_font_name(name: &str) -> bool {
  is_inside(name) && extension(name).is_some_and(|extension| FONT_EXTENSIONS.contains(&extension.as_str()))
}

fn is_sound_name(name: &str) -> bool {
  is_inside(name) && extension(name).is_some_and(|extension| SOUND_EXTENSIONS.contains(&extension.as_str()))
}

fn is_scene_name(name: &str) -> bool {
  is_inside(name) && extension(name).is_some_and(|extension| extension == file::EXTENSION)
}

/// Whether a name is a relative, `/`-separated path that can't escape the folder on any OS. Names come from scene files,
/// which anyone with console access can write, so they're untrusted.
fn is_inside(name: &str) -> bool {
  let safe = |part: &str| !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':']);
  name.split('/').all(safe)
}

fn extension(name: &str) -> Option<String> {
  name.rsplit_once('.').map(|(_, extension)| extension.to_ascii_lowercase())
}

/// Creates a new file, failing if it exists, along with any missing parent folders. Removes the file if the write fails.
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
  if let Some(folder) = path.parent() {
    fs::create_dir_all(folder)?;
  }
  let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
  file.write_all(bytes).inspect_err(|_| {
    let _ = fs::remove_file(path);
  })
}

/// Writes to `saving`, then renames it over `path`
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

/// Decodes a GIF frame by frame, stopping as soon as it passes a limit
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
  use crate::scene::{Fill, FillNode, Shape, ShapeNode};
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

  fn drawing(name: &str) -> Vec<Layer> {
    vec![Layer::new(Fill::new(FillNode::image(name)))]
  }

  #[test]
  fn images_are_found_however_the_folder_is_organized() {
    let folder = Folder::new();
    folder.write("title.png", &png(1, 1));
    folder.write("dmd/modes/Hurry Up.PNG", &png(1, 1));
    folder.write("photos/a.jpg", b"not looked at until it's used");
    folder.write("sounds/hit.mp3", b"");
    folder.write("attract.scene", b"");

    assert_eq!(
      Content::new(&folder.0).image_names(),
      ["dmd/modes/Hurry Up.PNG", "photos/a.jpg", "title.png"]
    );
  }

  #[test]
  fn sounds_are_found_by_their_extension() {
    let folder = Folder::new();
    folder.write("sounds/hit.mp3", b"");
    folder.write("sounds/callouts/Jackpot.WAV", b"");
    folder.write("title.png", &png(1, 1));
    folder.write("notes.txt", b"");

    let content = Content::new(&folder.0);
    assert_eq!(content.sound_names(), ["sounds/callouts/Jackpot.WAV", "sounds/hit.mp3"]);
    assert!(matches!(content.sound_path("../hit.mp3"), Err(ContentError::BadName(_))));
    assert!(matches!(content.sound_path("title.png"), Err(ContentError::BadName(_))));
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
    assert!(matches!(Content::new("root").image_path("attract.scene"), Err(ContentError::BadName(_))));
    assert!(Content::new("root").image_path("dmd/A.PNG").is_ok());
  }

  #[test]
  fn a_scene_gets_the_images_it_names() {
    let folder = Folder::new();
    folder.write("dmd/a.png", &png(3, 2));
    folder.write("unused.png", &png(1, 1));

    let images = Content::new(&folder.0).assets_for(&drawing("dmd/a.png")).unwrap();
    assert_eq!(images.picture("dmd/a.png").unwrap().width(), 3);
    assert!(images.picture("unused.png").is_none());
  }

  #[test]
  fn a_scene_naming_a_missing_image_says_which() {
    let folder = Folder::new();
    let error = Content::new(&folder.0).assets_for(&drawing("logo.png")).unwrap_err();
    assert!(error.to_string().contains("logo.png"), "{error}");
  }

  #[test]
  fn a_missing_image_is_blamed_on_the_layer_that_draws_it() {
    let folder = Folder::new();
    let mut scene = drawing("logo.png");
    assert_eq!(Content::new(&folder.0).assets_for(&scene).unwrap_err().layer, "layer 1");

    scene[0].name = Some("title".into());
    let error = Content::new(&folder.0).assets_for(&scene).unwrap_err();
    assert_eq!(error.to_string(), "layer \"title\": there's no image \"logo.png\" in the content folder");
  }

  const PIXEL_FONT: &[u8] = include_bytes!("../../tests/fonts/LCDBlock.ttf");

  fn writing(font: &str) -> Vec<Layer> {
    let shape = Shape::new(ShapeNode::text("HI", font, 8.0, glam::Vec2::new(16.0, 8.0)));
    vec![Layer::new(Fill::new(FillNode::image("a.png"))).region(shape)]
  }

  #[test]
  fn fonts_are_found_by_their_extension() {
    let folder = Folder::new();
    folder.write("fonts/pixel.ttf", PIXEL_FONT);
    folder.write("fonts/Other.OTF", PIXEL_FONT);
    folder.write("fonts/readme.txt", b"");
    assert_eq!(Content::new(&folder.0).font_names(), ["fonts/Other.OTF", "fonts/pixel.ttf"]);
  }

  #[test]
  fn a_scene_gets_the_fonts_its_text_is_in() {
    let folder = Folder::new();
    folder.write("a.png", &png(1, 1));
    folder.write("fonts/pixel.ttf", PIXEL_FONT);
    let assets = Content::new(&folder.0).assets_for(&writing("fonts/pixel.ttf")).unwrap();
    assert!(assets.font("fonts/pixel.ttf").is_some());
    assert!(assets.picture("a.png").is_some());
  }

  #[test]
  fn a_missing_font_is_blamed_on_the_layer_that_writes_in_it() {
    let folder = Folder::new();
    folder.write("a.png", &png(1, 1));
    let error = Content::new(&folder.0).assets_for(&writing("fonts/pixel.ttf")).unwrap_err();
    assert_eq!(error.to_string(), "layer 1: there's no font \"fonts/pixel.ttf\" in the content folder");
    assert!(matches!(Content::new(&folder.0).font("pixel.png"), Err(ContentError::BadFontName(_))));
  }

  #[test]
  fn fonts_given_to_it_are_found_before_files_and_listed_with_them() {
    let folder = Folder::new();
    folder.write("fonts/pixel.ttf", PIXEL_FONT);
    let pixel = Font::new(PIXEL_FONT.to_vec()).unwrap();
    let content = Content::new(&folder.0).with_fonts([("Pixel", pixel.clone())]).with_fonts([("Other", pixel)]);
    assert_eq!(content.font_names(), ["Other", "Pixel", "fonts/pixel.ttf"]);
    assert!(content.font("Pixel").is_ok());
    assert!(content.clone().font("Other").is_ok());
    assert!(matches!(content.font("Missing"), Err(ContentError::BadFontName(_))));
  }

  #[test]
  fn without_a_folder_it_has_only_the_fonts_its_given() {
    let pixel = Font::new(PIXEL_FONT.to_vec()).unwrap();
    let content = Content::without_folder().with_fonts([("Pixel", pixel)]);
    assert_eq!(content.font_names(), ["Pixel"]);
    assert!(content.font("Pixel").is_ok());
    assert!(content.image_names().is_empty() && content.scene_names().is_empty());
    assert!(matches!(content.font("fonts/pixel.ttf"), Err(ContentError::NoFolder)));
    assert!(matches!(content.picture("a.png"), Err(ContentError::NoFolder)));
    assert!(matches!(content.add_scene("a.scene", ""), Err(SceneFileError::NoFolder)));
    assert_eq!(content.read_scene("a.scene").unwrap_err().to_string(), "the app has no content folder");
  }

  #[test]
  fn a_file_that_is_not_a_font_says_which() {
    let folder = Folder::new();
    folder.write("a.ttf", b"not a font");
    assert!(matches!(Content::new(&folder.0).font("a.ttf"), Err(ContentError::NotAFont(_))));
  }

  #[test]
  fn a_font_is_loaded_again_once_its_file_changes() {
    let folder = Folder::new();
    folder.write("a.ttf", PIXEL_FONT);
    let content = Content::new(&folder.0);
    assert!(content.font("a.ttf").is_ok());
    folder.write("a.ttf", b"not a font now");
    assert!(matches!(content.font("a.ttf"), Err(ContentError::NotAFont(_))));
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
    folder.write("attract.scene", b"");
    folder.write("dmd/modes/Hurry Up.SCENE", b"");
    folder.write("title.png", &png(1, 1));

    assert_eq!(Content::new(&folder.0).scene_names(), ["attract.scene", "dmd/modes/Hurry Up.SCENE"]);
  }

  #[test]
  fn no_scene_name_leads_out_of_the_folder() {
    let content = Content::new("root");
    for name in ["../secret.scene", "a/../../secret.scene", "/etc/secret.scene", "C:/secret.scene", "a\\..\\secret.scene", "a.png", ""] {
      assert!(matches!(content.scene_path(name), Err(SceneFileError::BadName(_))), "{name}");
    }
  }

  #[test]
  fn an_added_scene_can_be_read_back_and_makes_its_folders() {
    let folder = Folder::new();
    let content = Content::new(folder.0.join("not/made/yet"));
    content.add_scene("led/a.scene", SCENE).unwrap();
    assert_eq!(content.scene_names(), ["led/a.scene"]);
    assert_eq!(content.read_scene("led/a.scene").unwrap(), SCENE);
  }

  #[test]
  fn adding_never_replaces_a_scene() {
    let folder = Folder::new();
    folder.write("a.scene", b"what was there");
    let content = Content::new(&folder.0);

    assert!(matches!(content.add_scene("a.scene", SCENE), Err(SceneFileError::Taken(_))));
    assert_eq!(content.read_scene("a.scene").unwrap(), "what was there");
  }

  #[test]
  fn saving_replaces_a_scene_and_leaves_nothing_else() {
    let folder = Folder::new();
    folder.write("a.scene", b"what was there");
    let content = Content::new(&folder.0);

    content.save_scene("a.scene", SCENE).unwrap();
    assert_eq!(content.read_scene("a.scene").unwrap(), SCENE);
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 1);
  }

  #[test]
  fn deleting_removes_only_that_scene() {
    let folder = Folder::new();
    folder.write("led/a.scene", b"");
    folder.write("led/b.scene", b"");
    let content = Content::new(&folder.0);

    content.delete_scene("led/a.scene").unwrap();
    assert_eq!(content.scene_names(), ["led/b.scene"]);
  }

  #[test]
  fn deleting_a_missing_scene_says_which() {
    let folder = Folder::new();
    let error = Content::new(&folder.0).delete_scene("a.scene").unwrap_err();
    assert_eq!(error.to_string(), "there's no scene \"a.scene\" in the content folder");
  }

  #[test]
  fn deleting_stays_in_the_folder() {
    let content = Content::new("root");
    assert!(matches!(content.delete_scene("../secret.scene"), Err(SceneFileError::BadName(_))));
  }

  #[test]
  fn a_broken_scene_can_still_be_opened() {
    let folder = Folder::new();
    folder.write("a.scene", b"(oops");
    assert_eq!(Content::new(&folder.0).read_scene("a.scene").unwrap(), "(oops");
  }

  #[test]
  fn a_missing_scene_says_which() {
    let folder = Folder::new();
    let error = Content::new(&folder.0).read_scene("a.scene").unwrap_err();
    assert_eq!(error.to_string(), "there's no scene \"a.scene\" in the content folder");
  }

  #[test]
  fn an_image_too_big_to_be_worth_decoding_is_refused() {
    let folder = Folder::new();
    let content = Content::new(&folder.0);
    let result = content.add_image("a.png", &png(MAX_IMAGE_SIDE + 1, 1));
    assert!(matches!(result, Err(ContentError::Decode { .. })));
  }
}
