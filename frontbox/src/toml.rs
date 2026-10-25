//! # TOML Files
//!
//! Reading and writing TOML files that hold saved state (operator config, high scores, etc.). A machine can lose power
//! at any moment, so a save never leaves a half-written file, and a file that fails to load is set aside rather than
//! being overwritten by the next save.
//!
//! ```rust,no_run
//! use frontbox::toml;
//! #[derive(Default, serde::Serialize, serde::Deserialize)]
//! #[serde(default)] // fields added later fall back to their default instead of failing the whole file
//! struct Scores {
//!   best: u32,
//! }
//!
//! let mut scores: Scores = toml::load("scores.toml").unwrap_or_default();
//! scores.best = 100;
//! toml::save("scores.toml", &scores).unwrap();
//! ```

use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Load a TOML file, returning `None` if it doesn't exist or can't be loaded.
///
/// A file that fails to parse is renamed to `<file>.bad` so the next [`save`] doesn't replace it with defaults, leaving
/// it to be inspected or fixed by hand.
pub fn load<T: DeserializeOwned>(path: impl AsRef<Path>) -> Option<T> {
  let path = path.as_ref();

  let text = match fs::read_to_string(path) {
    Ok(text) => text,
    Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
    Err(e) => {
      log::error!("Failed to read {}: {e}", path.display());
      return None;
    }
  };

  match toml::from_str(&text) {
    Ok(value) => Some(value),
    Err(e) => {
      let bad = with_suffix(path, "bad");
      log::error!(
        "Failed to parse {}, moving it to {}: {e}",
        path.display(),
        bad.display()
      );
      if let Err(e) = fs::rename(path, &bad) {
        log::error!("Failed to move {} aside: {e}", path.display());
      }
      None
    }
  }
}

/// Save a value as a TOML file, replacing the file atomically.
///
/// The value is written to `<file>.tmp`, flushed to disk, then renamed over the original, so after a power cut the file
/// holds either the complete old value or the complete new one.
pub fn save<T: Serialize>(path: impl AsRef<Path>, value: &T) -> io::Result<()> {
  let path = path.as_ref();
  let text = toml::to_string_pretty(value).map_err(io::Error::other)?;
  let tmp = with_suffix(path, "tmp");

  let written = write_synced(&tmp, text.as_bytes()).and_then(|_| fs::rename(&tmp, path));
  if written.is_err() {
    let _ = fs::remove_file(&tmp);
  }
  written?;

  sync_parent(path)
}

fn write_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
  let mut file = File::create(path)?;
  file.write_all(bytes)?;
  file.sync_all()
}

/// On Linux a rename isn't durable until the directory holding it is synced
#[cfg(unix)]
fn sync_parent(path: &Path) -> io::Result<()> {
  let dir = match path.parent() {
    Some(dir) if !dir.as_os_str().is_empty() => dir,
    _ => Path::new("."),
  };
  File::open(dir)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> io::Result<()> {
  Ok(())
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
  let mut name = OsString::from(path.as_os_str());
  name.push(".");
  name.push(suffix);
  PathBuf::from(name)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
  #[serde(default)]
  struct Saved {
    name: String,
    count: u32,
  }

  /// A fresh, empty directory per test
  fn test_dir(name: &str) -> PathBuf {
    let dir =
      std::env::temp_dir().join(format!("frontbox-toml-file-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
  }

  #[test]
  fn missing_file_loads_none() {
    let dir = test_dir("missing");
    assert_eq!(load::<Saved>(dir.join("saved.toml")), None);
  }

  #[test]
  fn round_trips() {
    let path = test_dir("round_trip").join("saved.toml");
    let saved = Saved {
      name: "AAA".into(),
      count: 3,
    };
    save(&path, &saved).unwrap();
    assert_eq!(load::<Saved>(&path), Some(saved));
  }

  #[test]
  fn save_replaces_existing_and_leaves_no_temp_file() {
    let path = test_dir("replace").join("saved.toml");
    save(
      &path,
      &Saved {
        name: "old".into(),
        count: 1,
      },
    )
    .unwrap();
    save(
      &path,
      &Saved {
        name: "new".into(),
        count: 2,
      },
    )
    .unwrap();

    assert_eq!(load::<Saved>(&path).unwrap().name, "new");
    assert!(!with_suffix(&path, "tmp").exists());
  }

  #[test]
  fn missing_fields_use_defaults() {
    let path = test_dir("defaults").join("saved.toml");
    fs::write(&path, "name = \"AAA\"\n").unwrap();
    assert_eq!(
      load::<Saved>(&path),
      Some(Saved {
        name: "AAA".into(),
        count: 0
      })
    );
  }

  #[test]
  fn unparseable_file_is_set_aside() {
    let path = test_dir("bad").join("saved.toml");
    fs::write(&path, "count = \"not a number\"\n").unwrap();

    assert_eq!(load::<Saved>(&path), None);
    assert!(!path.exists());
    assert_eq!(
      fs::read_to_string(with_suffix(&path, "bad")).unwrap(),
      "count = \"not a number\"\n"
    );
  }
}
