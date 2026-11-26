use std::any::{Any, TypeId};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

/// Everything an app is made with that isn't a system: what's in its content folder, and what its code adds
///
/// Each [`Resource`] is a marker type for one sort of thing, such as fonts, whose items are looked up by name. It says
/// what an item is and, for a resource kept in files, how a file becomes one. Core has no resources of its own; each
/// crate declares its own, such as frontbox-scene's fonts, images, and scenes.
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// struct Greetings;
///
/// impl Resource for Greetings {
///   type Item = String;
///   const NAME: &'static str = "greeting";
/// }
///
/// let mut resources = Resources::without_folder();
/// resources.add::<Greetings>("hello", "Hello!".into());
/// assert_eq!(resources.get::<Greetings>("hello").unwrap(), "Hello!");
/// ```
///
/// Items get in two ways:
///
/// - **Files** in the content folder ([`BootConfig::content_path`](crate::app::BootConfig::content_path)). The folder
///   is scanned recursively, so it can be organized freely and hold other files too. A file is named by its path from
///   the folder with forward slashes, such as `dmd/title.png`, so names are the same on every OS, and its extension
///   says which resources it can be an item of. Files are loaded when first asked for and kept until they change.
/// - **Code**, with [`Self::add`], while the app is built ([`App::resources`](crate::app::App::resources)): what a crate
///   comes with, and what can't be a file at all.
///
/// A name is the path of the file that would replace it: an item added to a resource kept in files is named as a file
/// of it would be (`frontbox/fonts/TinyPixie2.ttf`), and a file at that path in the folder is used in its place.
/// That's how a game changes what a crate comes with. A resource that's never a file has plain names (`tentacle`).
///
/// What's added is fixed once the app runs, so anything with state, a lifecycle, or events is a system instead.
/// Systems reach it as `ctx.resources`.
#[derive(Default)]
pub struct Resources {
  /// `None` for an app without a content folder
  root: Option<PathBuf>,
  /// What code added: a `BTreeMap<String, R::Item>` for each resource `R`, by its type
  added: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
  /// What's been loaded from files, by resource and then name
  loaded: Mutex<HashMap<TypeId, HashMap<String, Loaded>>>,
}

impl Resources {
  /// With the content folder at `root`, which doesn't have to be there yet
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self::in_folder(Some(root.into()))
  }

  /// For an app without a content folder, which has only what code adds
  pub fn without_folder() -> Self {
    Self::default()
  }

  /// With the content folder at `root`, or without one
  pub fn in_folder(root: Option<PathBuf>) -> Self {
    Self {
      root,
      ..Self::default()
    }
  }

  /// The content folder. `None` for an app without one.
  pub fn root(&self) -> Option<&Path> {
    self.root.as_deref()
  }

  /// Adds an item from code. One added again by the same name replaces the one before.
  ///
  /// # Panics
  ///
  /// For a resource kept in files, when `name` isn't the path a file of it could have, since nothing could ever ask for
  /// it.
  pub fn add<R: Resource>(&mut self, name: impl Into<String>, item: R::Item) -> &mut Self {
    let name = name.into();
    assert!(
      is_name::<R>(&name),
      "{}",
      ResourceError::BadName {
        resource: R::NAME,
        name,
        extensions: R::EXTENSIONS
      }
    );
    let added = self
      .added
      .entry(TypeId::of::<R>())
      .or_insert_with(|| Box::new(BTreeMap::<String, R::Item>::new()))
      .downcast_mut::<BTreeMap<String, R::Item>>()
      .expect("a resource's added items are kept by its type");
    if added.contains_key(&name) {
      log::warn!(target: "frontbox::resources", "{} \"{name}\" was added twice; the last one is used", R::NAME);
    }
    added.insert(name, item);
    self
  }

  /// The item named `name`: its file in the content folder, or else the one code added. Clones are meant to be cheap,
  /// so a large item is kept behind an `Arc`.
  ///
  /// A file is read and loaded the first time, and again once it changes, which is slow, so don't call this in a tick.
  pub fn get<R: Resource>(&self, name: &str) -> Result<R::Item, ResourceError> {
    let added = self.added_items::<R>().and_then(|added| added.get(name));
    if R::EXTENSIONS.is_empty() {
      return added.cloned().ok_or_else(|| ResourceError::NotAdded {
        resource: R::NAME,
        name: name.into(),
      });
    }
    let path = match (self.path::<R>(name), added) {
      (Ok(path), _) => path,
      (Err(ResourceError::NoFolder { .. }), Some(added)) => return Ok(added.clone()),
      (Err(error), _) => return Err(error),
    };
    match stamp(&path) {
      Ok(stamp) => self.load::<R>(name, &path, stamp),
      Err(error) => {
        self.forget::<R>(name);
        match (error.kind(), added) {
          (io::ErrorKind::NotFound, Some(added)) => Ok(added.clone()),
          (io::ErrorKind::NotFound, None) => Err(ResourceError::Missing {
            resource: R::NAME,
            name: name.into(),
          }),
          _ => Err(ResourceError::Read {
            resource: R::NAME,
            name: name.into(),
            error,
          }),
        }
      }
    }
  }

  /// Every name a resource has, sorted: its files in the content folder, and what code added
  pub fn names<R: Resource>(&self) -> Vec<String> {
    let mut names: Vec<String> = self.added::<R>().map(|(name, _)| name.to_owned()).collect();
    if let Some(root) = &self.root
      && !R::EXTENSIONS.is_empty()
    {
      scan(root, "", R::EXTENSIONS, &mut names);
    }
    names.sort();
    names.dedup();
    names
  }

  /// What code added to a resource, sorted by name, whether or not a file replaces it
  pub fn added<R: Resource>(&self) -> impl Iterator<Item = (&str, &R::Item)> {
    let added = self.added_items::<R>().into_iter().flatten();
    added.map(|(name, item)| (name.as_str(), item))
  }

  /// Where the file named `name` is, or would be, in the content folder. Doesn't check that it's there.
  pub fn path<R: Resource>(&self, name: &str) -> Result<PathBuf, ResourceError> {
    if R::EXTENSIONS.is_empty() || !is_name::<R>(name) {
      return Err(ResourceError::BadName {
        resource: R::NAME,
        name: name.into(),
        extensions: R::EXTENSIONS,
      });
    }
    let root = self.root.as_ref().ok_or_else(|| ResourceError::NoFolder {
      resource: R::NAME,
      name: name.into(),
    })?;
    Ok(root.join(name))
  }

  /// Writes a new file to the content folder, with any folders it needs. One that's there is never replaced, since
  /// something may be using it. `bytes` aren't checked to be what the resource loads.
  pub fn create<R: Resource>(&self, name: &str, bytes: &[u8]) -> Result<(), ResourceError> {
    let path = self.path::<R>(name)?;
    write_new(&path, bytes).map_err(|error| match error.kind() {
      io::ErrorKind::AlreadyExists => ResourceError::Taken {
        resource: R::NAME,
        name: name.into(),
      },
      _ => ResourceError::Write {
        resource: R::NAME,
        name: name.into(),
        error,
      },
    })
  }

  /// Writes a file to the content folder, replacing any by that name. It writes a temp file and renames it into
  /// place, so readers never see a partial file and a failed save loses nothing.
  pub fn save<R: Resource>(&self, name: &str, bytes: &[u8]) -> Result<(), ResourceError> {
    let path = self.path::<R>(name)?;
    let mut saving = path.clone().into_os_string();
    saving.push(".saving");
    let saving = PathBuf::from(saving);
    replace(&path, &saving, bytes).map_err(|error| {
      let _ = fs::remove_file(&saving);
      ResourceError::Write {
        resource: R::NAME,
        name: name.into(),
        error,
      }
    })
  }

  /// Deletes a file from the content folder. Its parent folders are left, even if empty, and so is anything code added
  /// by that name.
  pub fn delete<R: Resource>(&self, name: &str) -> Result<(), ResourceError> {
    let path = self.path::<R>(name)?;
    fs::remove_file(path).map_err(|error| match error.kind() {
      io::ErrorKind::NotFound => ResourceError::Missing {
        resource: R::NAME,
        name: name.into(),
      },
      _ => ResourceError::Delete {
        resource: R::NAME,
        name: name.into(),
        error,
      },
    })
  }

  fn added_items<R: Resource>(&self) -> Option<&BTreeMap<String, R::Item>> {
    let added = self.added.get(&TypeId::of::<R>())?;
    Some(
      added
        .downcast_ref()
        .expect("a resource's added items are kept by its type"),
    )
  }

  /// The item in the file at `path`, last written at `stamp`: the one loaded before, unless the file has changed since
  fn load<R: Resource>(
    &self,
    name: &str,
    path: &Path,
    stamp: Stamp,
  ) -> Result<R::Item, ResourceError> {
    let resource = TypeId::of::<R>();
    if let Some(loaded) = self
      .loaded
      .lock()
      .unwrap()
      .get(&resource)
      .and_then(|loaded| loaded.get(name))
      .filter(|loaded| loaded.stamp == stamp)
    {
      return Ok(loaded.item::<R>().clone());
    }

    // not while the lock is held, as loading can be slow
    let bytes = fs::read(path).map_err(|error| ResourceError::Read {
      resource: R::NAME,
      name: name.into(),
      error,
    })?;
    let item = R::load(name, bytes).map_err(|error| ResourceError::Load {
      resource: R::NAME,
      name: name.into(),
      error,
    })?;
    let loaded = Loaded {
      stamp,
      size: R::size(&item),
      item: Box::new(item.clone()),
    };
    keep(
      self.loaded.lock().unwrap().entry(resource).or_default(),
      name,
      loaded,
      R::KEPT_SIZE,
    );
    Ok(item)
  }

  /// Lets go of what was loaded from a file that's no longer there
  fn forget<R: Resource>(&self, name: &str) {
    if let Some(loaded) = self.loaded.lock().unwrap().get_mut(&TypeId::of::<R>()) {
      loaded.remove(name);
    }
  }
}

impl fmt::Debug for Resources {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("Resources")
      .field("root", &self.root)
      .field("added_to", &self.added.len())
      .finish_non_exhaustive()
  }
}

/// One sort of thing in an app's [`Resources`], such as its fonts, whose items are looked up by name. It's implemented
/// by a marker type that's only ever named, never made:
///
/// ```rust
/// # use frontbox_core::prelude::*;
/// /// Text files, by their path in the content folder
/// pub struct Notes;
///
/// impl Resource for Notes {
///   type Item = String;
///   const NAME: &'static str = "note";
///   const EXTENSIONS: &'static [&'static str] = &["txt"];
///
///   fn load(_name: &str, bytes: Vec<u8>) -> Result<String, String> {
///     String::from_utf8(bytes).map_err(|error| error.to_string())
///   }
/// }
/// ```
///
/// A marker rather than the item's own type, so two resources can share an item type, and a crate can declare one of
/// another crate's type.
pub trait Resource: 'static {
  /// What one is. It's cloned each time it's asked for, so keep anything large behind an `Arc`.
  type Item: Clone + Send + Sync + 'static;

  /// What one is called in errors, in lowercase: `"font"`
  const NAME: &'static str;

  /// The extensions of the files one can be loaded from, in lowercase. Empty for a resource that's only added by code.
  const EXTENSIONS: &'static [&'static str] = &[];

  /// The total [`Self::size`] of loaded files kept at once. Past it, the others are let go of, and loaded again when
  /// next asked for. Those in use stay alive for whatever holds them.
  const KEPT_SIZE: u64 = u64::MAX;

  /// An item from the contents of the file named `name`. The error says why it couldn't be one.
  ///
  /// A resource whose files are only listed and found ([`Resources::names`], [`Resources::path`]) leaves this out.
  fn load(name: &str, bytes: Vec<u8>) -> Result<Self::Item, String> {
    let _ = (name, bytes);
    Err(format!(
      "each {}'s file is only found, not loaded",
      Self::NAME
    ))
  }

  /// How much memory a loaded item takes, in bytes, for [`Self::KEPT_SIZE`]
  fn size(item: &Self::Item) -> u64 {
    let _ = item;
    0
  }
}

/// Why an item in an app's [`Resources`] can't be used. `resource` is its [`Resource::NAME`].
#[derive(Debug)]
pub enum ResourceError {
  /// It isn't in the content folder, and code added none by that name
  Missing {
    resource: &'static str,
    name: String,
  },
  /// The app has no content folder for its file to be in, and code added none by that name
  NoFolder {
    resource: &'static str,
    name: String,
  },
  /// Code added none by that name, to a resource that's only added by code
  NotAdded {
    resource: &'static str,
    name: String,
  },
  /// Not a path inside the content folder that a file of this resource could have
  BadName {
    resource: &'static str,
    name: String,
    extensions: &'static [&'static str],
  },
  Read {
    resource: &'static str,
    name: String,
    error: io::Error,
  },
  /// Its file isn't what the resource loads, for why
  Load {
    resource: &'static str,
    name: String,
    error: String,
  },
  /// A file by this name is already there
  Taken {
    resource: &'static str,
    name: String,
  },
  Write {
    resource: &'static str,
    name: String,
    error: io::Error,
  },
  Delete {
    resource: &'static str,
    name: String,
    error: io::Error,
  },
}

impl fmt::Display for ResourceError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Missing { resource, name } => {
        write!(f, "there's no {resource} \"{name}\" in the content folder")
      }
      Self::NoFolder { resource, name } => write!(
        f,
        "there's no {resource} \"{name}\": the app has no content folder"
      ),
      Self::NotAdded { resource, name } => write!(f, "the app has no {resource} \"{name}\""),
      Self::BadName {
        resource,
        name,
        extensions: [],
      } => write!(f, "\"{name}\" can't be the name of any {resource}"),
      Self::BadName {
        resource,
        name,
        extensions,
      } => write!(
        f,
        "\"{name}\" can't be the name of any {resource}, which is a path within the content folder ending in .{}",
        extensions.join(" or .")
      ),
      Self::Read {
        resource,
        name,
        error,
      } => write!(f, "couldn't read {resource} \"{name}\": {error}"),
      Self::Load {
        resource,
        name,
        error,
      } => write!(f, "couldn't load {resource} \"{name}\": {error}"),
      Self::Taken { resource, name } => {
        write!(f, "there's already a file for {resource} \"{name}\"")
      }
      Self::Write {
        resource,
        name,
        error,
      } => write!(f, "couldn't save {resource} \"{name}\": {error}"),
      Self::Delete {
        resource,
        name,
        error,
      } => {
        write!(f, "couldn't delete {resource} \"{name}\": {error}")
      }
    }
  }
}

impl std::error::Error for ResourceError {}

/// An item loaded from a file
struct Loaded {
  /// The file's when it was loaded
  stamp: Stamp,
  /// Its [`Resource::size`]
  size: u64,
  /// An `R::Item`, for the resource `R` it's kept under
  item: Box<dyn Any + Send + Sync>,
}

impl Loaded {
  fn item<R: Resource>(&self) -> &R::Item {
    self
      .item
      .downcast_ref()
      .expect("a loaded item is kept by its resource's type")
  }
}

/// A file's modified time and size, which change when it's written
type Stamp = (SystemTime, u64);

fn stamp(path: &Path) -> io::Result<Stamp> {
  let metadata = fs::metadata(path)?;
  Ok((metadata.modified()?, metadata.len()))
}

/// Keeps a loaded item, letting go of all the others of its resource if their total size would pass `limit`
fn keep(loaded: &mut HashMap<String, Loaded>, name: &str, added: Loaded, limit: u64) {
  loaded.remove(name);
  let total = loaded
    .values()
    .map(|loaded| loaded.size)
    .fold(added.size, u64::saturating_add);
  if total > limit {
    loaded.clear();
  }
  loaded.insert(name.into(), added);
}

/// Whether `name` can name an item of a resource: for one kept in files, the path a file of it could have
fn is_name<R: Resource>(name: &str) -> bool {
  match R::EXTENSIONS.is_empty() {
    true => !name.is_empty(),
    false => is_inside(name) && has_extension(name, R::EXTENSIONS),
  }
}

/// Whether a name is a relative, `/`-separated path that can't escape the folder on any OS. Names come from scene files,
/// which anyone with console access can write, so they're untrusted.
fn is_inside(name: &str) -> bool {
  let safe =
    |part: &str| !part.is_empty() && part != "." && part != ".." && !part.contains(['\\', ':']);
  name.split('/').all(safe)
}

fn has_extension(name: &str, extensions: &[&str]) -> bool {
  name
    .rsplit_once('.')
    .is_some_and(|(_, extension)| extensions.contains(&extension.to_ascii_lowercase().as_str()))
}

/// Recursively collects the names of files with one of `extensions`. Symlinks aren't followed, to avoid cycles.
fn scan(folder: &Path, prefix: &str, extensions: &[&str], names: &mut Vec<String>) {
  let Ok(entries) = fs::read_dir(folder) else {
    return;
  };
  for entry in entries.flatten() {
    let Some(file_name) = entry.file_name().to_str().map(str::to_owned) else {
      continue;
    };
    let name = format!("{prefix}{file_name}");
    match entry.file_type() {
      Ok(kind) if kind.is_dir() => scan(&entry.path(), &format!("{name}/"), extensions, names),
      Ok(kind) if kind.is_file() && has_extension(&name, extensions) => names.push(name),
      _ => {}
    }
  }
}

/// Creates a new file, failing if it exists, along with any missing parent folders. Removes the file if the write fails.
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
  if let Some(folder) = path.parent() {
    fs::create_dir_all(folder)?;
  }
  let mut file = fs::OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(path)?;
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

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::Arc;
  use std::sync::atomic::{AtomicU32, Ordering};

  static NEXT_FOLDER: AtomicU32 = AtomicU32::new(0);

  /// An empty folder of its own, removed afterward
  struct Folder(PathBuf);

  impl Folder {
    fn new() -> Self {
      let id = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
      let path =
        std::env::temp_dir().join(format!("frontbox-resources-{}-{id}", std::process::id()));
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

  /// Text files. One that starts with `!` doesn't load.
  struct Notes;

  impl Resource for Notes {
    type Item = Arc<String>;
    const NAME: &'static str = "note";
    const EXTENSIONS: &'static [&'static str] = &["txt", "md"];
    const KEPT_SIZE: u64 = 8;

    fn load(_name: &str, bytes: Vec<u8>) -> Result<Arc<String>, String> {
      match bytes.first() {
        Some(b'!') => Err("it starts with !".into()),
        _ => Ok(Arc::new(String::from_utf8(bytes).unwrap())),
      }
    }

    fn size(item: &Arc<String>) -> u64 {
      item.len() as u64
    }
  }

  /// The same files as [`Notes`], only found
  struct Attachments;

  impl Resource for Attachments {
    type Item = ();
    const NAME: &'static str = "attachment";
    const EXTENSIONS: &'static [&'static str] = &["txt"];
  }

  /// Only added by code
  struct Tools;

  impl Resource for Tools {
    type Item = u32;
    const NAME: &'static str = "tool";
  }

  /// The same items as [`Tools`], as another resource
  struct Toys;

  impl Resource for Toys {
    type Item = u32;
    const NAME: &'static str = "toy";
  }

  fn note(text: &str) -> Arc<String> {
    Arc::new(text.to_owned())
  }

  #[test]
  fn what_code_adds_is_found_by_resource_and_name() {
    let mut resources = Resources::without_folder();
    resources.add::<Tools>("hammer", 1).add::<Tools>("anvil", 2);
    resources.add::<Toys>("hammer", 3);

    assert_eq!(resources.get::<Tools>("hammer").unwrap(), 1);
    assert_eq!(resources.get::<Toys>("hammer").unwrap(), 3);
    assert_eq!(resources.names::<Tools>(), ["anvil", "hammer"]);
    assert_eq!(
      resources.added::<Tools>().collect::<Vec<_>>(),
      [("anvil", &2), ("hammer", &1)]
    );
    assert_eq!(resources.added::<Notes>().count(), 0);
    assert_eq!(
      resources.get::<Tools>("saw").unwrap_err().to_string(),
      "the app has no tool \"saw\""
    );
  }

  #[test]
  fn one_added_again_replaces_the_one_before() {
    let mut resources = Resources::without_folder();
    resources
      .add::<Tools>("hammer", 1)
      .add::<Tools>("hammer", 2);
    assert_eq!(resources.get::<Tools>("hammer").unwrap(), 2);
    assert_eq!(resources.names::<Tools>(), ["hammer"]);
  }

  #[test]
  fn files_are_found_however_the_folder_is_organized() {
    let folder = Folder::new();
    folder.write("readme.txt", b"");
    folder.write("docs/deep/Guide.MD", b"");
    folder.write("title.png", b"");
    let resources = Resources::new(&folder.0);

    assert_eq!(
      resources.names::<Notes>(),
      ["docs/deep/Guide.MD", "readme.txt"]
    );
    assert_eq!(resources.names::<Attachments>(), ["readme.txt"]);
    assert!(resources.names::<Tools>().is_empty());
  }

  #[test]
  fn a_folder_that_is_not_there_has_no_files() {
    let resources = Resources::new("there/is/no/such/folder");
    assert!(resources.names::<Notes>().is_empty());
    assert_eq!(
      resources.get::<Notes>("a.txt").unwrap_err().to_string(),
      "there's no note \"a.txt\" in the content folder"
    );
  }

  #[test]
  fn no_name_leads_out_of_the_folder() {
    let resources = Resources::new("root");
    for name in [
      "../secret.txt",
      "a/../../secret.txt",
      "/etc/secret.txt",
      "C:/secret.txt",
      "a\\..\\secret.txt",
      "./a.txt",
      "a//b.txt",
      "a.png",
      "",
    ] {
      for result in [
        resources.path::<Notes>(name).map(|_| ()),
        resources.get::<Notes>(name).map(|_| ()),
        resources.create::<Notes>(name, b""),
        resources.save::<Notes>(name, b""),
        resources.delete::<Notes>(name),
      ] {
        assert!(
          matches!(result, Err(ResourceError::BadName { .. })),
          "{name}"
        );
      }
    }
    assert_eq!(
      resources.path::<Notes>("docs/A.TXT").unwrap(),
      Path::new("root").join("docs/A.TXT")
    );
    assert_eq!(
      resources.path::<Notes>("a.png").unwrap_err().to_string(),
      "\"a.png\" can't be the name of any note, which is a path within the content folder ending in .txt or .md"
    );
  }

  #[test]
  #[should_panic(expected = "\"hello\" can't be the name of any note")]
  fn one_added_to_a_resource_kept_in_files_is_named_as_a_file_would_be() {
    Resources::without_folder().add::<Notes>("hello", note("hi"));
  }

  #[test]
  fn a_file_is_loaded_once_until_it_changes() {
    let folder = Folder::new();
    folder.write("a.txt", b"one");
    let resources = Resources::new(&folder.0);

    let first = resources.get::<Notes>("a.txt").unwrap();
    assert_eq!(*first, "one");
    assert!(Arc::ptr_eq(
      &first,
      &resources.get::<Notes>("a.txt").unwrap()
    ));

    // a different size, so the file changes even where the clock is too coarse to see it
    folder.write("a.txt", b"three");
    assert_eq!(*resources.get::<Notes>("a.txt").unwrap(), "three");

    folder.write("a.txt", b"!");
    assert_eq!(
      resources.get::<Notes>("a.txt").unwrap_err().to_string(),
      "couldn't load note \"a.txt\": it starts with !"
    );
  }

  #[test]
  fn a_resource_that_only_finds_its_files_cant_load_one() {
    let folder = Folder::new();
    folder.write("a.txt", b"one");
    let resources = Resources::new(&folder.0);
    assert!(resources.path::<Attachments>("a.txt").unwrap().is_file());
    assert!(matches!(
      resources.get::<Attachments>("a.txt"),
      Err(ResourceError::Load { .. })
    ));
  }

  #[test]
  fn a_file_replaces_what_code_added_by_its_name() {
    let folder = Folder::new();
    let mut resources = Resources::new(&folder.0);
    resources.add::<Notes>("crate/a.txt", note("added"));
    assert_eq!(*resources.get::<Notes>("crate/a.txt").unwrap(), "added");

    folder.write("crate/a.txt", b"changed");
    assert_eq!(*resources.get::<Notes>("crate/a.txt").unwrap(), "changed");
    assert_eq!(resources.names::<Notes>(), ["crate/a.txt"]);
    assert_eq!(resources.added::<Notes>().count(), 1);

    resources.delete::<Notes>("crate/a.txt").unwrap();
    assert_eq!(*resources.get::<Notes>("crate/a.txt").unwrap(), "added");
  }

  #[test]
  fn without_a_folder_it_has_only_what_code_added() {
    let mut resources = Resources::without_folder();
    resources.add::<Notes>("crate/a.txt", note("added"));

    assert_eq!(resources.names::<Notes>(), ["crate/a.txt"]);
    assert_eq!(*resources.get::<Notes>("crate/a.txt").unwrap(), "added");
    assert_eq!(
      resources.get::<Notes>("b.txt").unwrap_err().to_string(),
      "there's no note \"b.txt\": the app has no content folder"
    );
    assert!(matches!(
      resources.create::<Notes>("b.txt", b""),
      Err(ResourceError::NoFolder { .. })
    ));
  }

  #[test]
  fn a_created_file_can_be_read_back_and_makes_its_folders() {
    let folder = Folder::new();
    let resources = Resources::new(folder.0.join("not/made/yet"));
    resources.create::<Notes>("docs/a.txt", b"new").unwrap();
    assert_eq!(resources.names::<Notes>(), ["docs/a.txt"]);
    assert_eq!(*resources.get::<Notes>("docs/a.txt").unwrap(), "new");
  }

  #[test]
  fn creating_never_replaces_a_file() {
    let folder = Folder::new();
    folder.write("a.txt", b"what was there");
    let resources = Resources::new(&folder.0);

    assert!(matches!(
      resources.create::<Notes>("a.txt", b"new"),
      Err(ResourceError::Taken { .. })
    ));
    assert_eq!(*resources.get::<Notes>("a.txt").unwrap(), "what was there");
  }

  #[test]
  fn saving_replaces_a_file_and_leaves_nothing_else() {
    let folder = Folder::new();
    folder.write("a.txt", b"what was there");
    let resources = Resources::new(&folder.0);

    resources.save::<Notes>("a.txt", b"new").unwrap();
    assert_eq!(*resources.get::<Notes>("a.txt").unwrap(), "new");
    assert_eq!(fs::read_dir(&folder.0).unwrap().count(), 1);
  }

  #[test]
  fn deleting_removes_only_that_file() {
    let folder = Folder::new();
    folder.write("docs/a.txt", b"");
    folder.write("docs/b.txt", b"");
    let resources = Resources::new(&folder.0);

    resources.delete::<Notes>("docs/a.txt").unwrap();
    assert_eq!(resources.names::<Notes>(), ["docs/b.txt"]);
    assert_eq!(
      resources
        .delete::<Notes>("docs/a.txt")
        .unwrap_err()
        .to_string(),
      "there's no note \"docs/a.txt\" in the content folder"
    );
  }

  #[test]
  fn loaded_files_are_let_go_of_past_the_resources_limit() {
    let folder = Folder::new();
    folder.write("a.txt", b"1234");
    folder.write("b.txt", b"1234");
    folder.write("c.txt", b"1");
    let resources = Resources::new(&folder.0);
    let kept = |name| {
      let loaded = resources.loaded.lock().unwrap();
      loaded[&TypeId::of::<Notes>()].contains_key(name)
    };

    let a = resources.get::<Notes>("a.txt").unwrap();
    resources.get::<Notes>("b.txt").unwrap();
    assert!(kept("a.txt") && kept("b.txt"));

    // a new version of one takes that one's place
    folder.write("b.txt", b"123");
    resources.get::<Notes>("b.txt").unwrap();
    assert!(kept("a.txt") && kept("b.txt"));

    resources.get::<Notes>("a.txt").unwrap();
    resources.get::<Notes>("c.txt").unwrap();
    assert!(kept("a.txt") && kept("b.txt") && kept("c.txt"));

    folder.write("b.txt", b"12345");
    resources.get::<Notes>("b.txt").unwrap();
    assert!(!kept("a.txt") && kept("b.txt") && !kept("c.txt"));
    assert_eq!(*a, "1234", "one in use stays alive");
  }
}
