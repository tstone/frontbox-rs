//! Rewrites version 1 scene files, `Led(...)` and `Dmd(...)`, as version 2 `Scene(...)` files, in place. Every `.scene`
//! file under the folder given is read; ones already in version 2 are left alone. Comments in a file aren't kept.
//!
//! ```text
//! cargo run -p frontbox-scene --example convert_v1_scenes -- path/to/assets
//! ```

use std::path::{Path, PathBuf};
use std::{env, fs};

use frontbox_gfx::scene::file::{self, EXTENSION};
use frontbox_gfx::scene::{Layer, Params, Timing};
use frontbox_scene::{DisplayLayer, LedLayer, Scene, SceneLayer};
use serde::Deserialize;

/// What a version 1 DMD scene's layers became: one display, by this name
const DMD: &str = "Dmd";

fn main() {
  let Some(folder) = env::args().nth(1) else {
    eprintln!("usage: convert_v1_scenes <folder>");
    std::process::exit(2);
  };
  let mut failed = false;
  for path in scene_files(Path::new(&folder)) {
    let source = fs::read_to_string(&path).unwrap();
    if Scene::load(&source).is_ok() {
      println!("already version 2: {}", path.display());
      continue;
    }
    match convert(&source) {
      Ok(written) => {
        fs::write(&path, written).unwrap();
        println!("converted: {}", path.display());
      }
      Err(error) => {
        eprintln!("couldn't convert {}: {error}", path.display());
        failed = true;
      }
    }
  }
  if failed {
    std::process::exit(1);
  }
}

fn convert(source: &str) -> Result<String, String> {
  let scene = match file::load::<V1>(source).map_err(|error| error.to_string())? {
    V1::Led(led) => Scene { params: led.params, timing: led.timing, ..Scene::new(led.layers.into_iter().map(SceneLayer::from).collect()) },
    V1::Dmd(dmd) => {
      let layers = dmd.composition.layers.into_iter().map(|layer| DisplayLayer::new(DMD, layer).into()).collect();
      Scene { params: dmd.composition.params, timing: dmd.composition.timing, ..Scene::new(layers) }
    }
  };
  scene.check().map_err(|error| error.to_string())?;
  scene.save().map_err(|error| error.to_string())
}

fn scene_files(folder: &Path) -> Vec<PathBuf> {
  let mut found = Vec::new();
  for entry in fs::read_dir(folder).unwrap().flatten() {
    let path = entry.path();
    if path.is_dir() {
      found.extend(scene_files(&path));
    } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION)) {
      found.push(path);
    }
  }
  found
}

/// A version 1 file. An LED scene's layers are written the same in both versions.
#[derive(Deserialize)]
enum V1 {
  Led(LedV1),
  Dmd(DmdV1),
}

#[derive(Deserialize)]
struct LedV1 {
  #[serde(default)]
  params: Params,
  #[serde(default)]
  timing: Option<Timing>,
  #[serde(default)]
  layers: Vec<LedLayer>,
}

#[derive(Deserialize)]
struct DmdV1 {
  composition: CompositionV1,
}

#[derive(Deserialize)]
struct CompositionV1 {
  #[serde(default)]
  params: Params,
  #[serde(default)]
  timing: Option<Timing>,
  #[serde(default)]
  layers: Vec<Layer>,
}
