use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_FOLDER: AtomicU32 = AtomicU32::new(0);

/// An empty folder of its own, for a test's content folder, removed afterward
pub(crate) struct Folder(pub PathBuf);

impl Folder {
  pub fn new() -> Self {
    let id = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("frontbox-scene-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    Self(path)
  }

  pub fn write(&self, name: &str, bytes: &[u8]) {
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
