//! Builds the web app into `web/dist` when its sources change, so `cargo build` is all the console needs.
//!
//! Node is run directly on the local `vite` script rather than through `npm run build`, for two reasons:
//! there's no shell involved (`npm` is a `.cmd`/`.ps1` shim on Windows, and PowerShell's execution policy can refuse
//! it), and that script's `prebuild` step runs `cargo run --example export_types`, which would wait forever on the
//! build directory this build holds. Regenerating the TS types stays a separate step; see CONTRIBUTING.md.
//!
//! Node doesn't have to be on the PATH: nvm only sets it up for the shell it's loaded in, so a build started from
//! another shell or the editor looks in the usual install locations for a version Vite supports.
//!
//! Without a suitable Node the build warns and embeds whatever `web/dist` already holds. Set
//! `FRONTBOX_SKIP_WEB_BUILD` to skip building the web app entirely.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

/// What the web app is built from. A change to any of these rebuilds it.
const SOURCES: &[&str] = &[
  "src",
  "public",
  "index.html",
  "package.json",
  "package-lock.json",
  "vite.config.ts",
  "tsconfig.json",
  "tsconfig.app.json",
  "tsconfig.node.json",
];

fn main() {
  let web = Path::new(env!("CARGO_MANIFEST_DIR")).join("web");
  let dist = web.join("dist");

  println!("cargo:rerun-if-env-changed=FRONTBOX_SKIP_WEB_BUILD");
  // rust-embed decides at compile time what web/dist holds, so rebuild when it changes, e.g. from `npm run build`
  println!("cargo:rerun-if-changed={}", dist.display());
  for source in SOURCES {
    println!("cargo:rerun-if-changed={}", web.join(source).display());
  }

  if std::env::var_os("FRONTBOX_SKIP_WEB_BUILD").is_some() {
    return;
  }
  // a packaged crate ships only the built app
  if !web.join("src").exists() {
    return;
  }
  if !is_stale(&web, &dist) {
    return;
  }

  let Some(node) = find_node() else {
    println!(
      "cargo:warning=not building the web console: no Node.js 20.19+ or 22.12+ found, which Vite needs. Embedding the \
       existing web/dist, which is out of date."
    );
    // keep warning on every build until it's fixed, rather than only on the first
    println!("cargo:rerun-if-changed={}", web.join(".no-such-file").display());
    return;
  };

  if !web.join("node_modules").exists() {
    run(node.npm(&web).arg("ci"), "npm ci");
  }
  // No `tsc -b`: the generated TS types are written by an example in this crate, so a type check here would fail on
  // stale types before they could be regenerated. Vite strips types without checking them; `npm run build` and the
  // editor still check.
  run(
    node
      .command(&web)
      .args(["node_modules/vite/bin/vite.js", "build", "--logLevel", "warn"]),
    "vite build",
  );
}

/// Whether any source is newer than the last build. A build writes web/dist, which reruns this script, so without
/// this every build would also be followed by another.
fn is_stale(web: &Path, dist: &Path) -> bool {
  let Some(built) = modified(&dist.join("index.html")) else {
    return true;
  };
  SOURCES
    .iter()
    .filter_map(|source| newest(&web.join(source)))
    .any(|changed| changed > built)
}

fn newest(path: &Path) -> Option<SystemTime> {
  if !path.is_dir() {
    return modified(path);
  }
  std::fs::read_dir(path)
    .ok()?
    .filter_map(|entry| entry.ok())
    .filter_map(|entry| newest(&entry.path()))
    .max()
}

fn modified(path: &Path) -> Option<SystemTime> {
  std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

struct Node {
  exe: PathBuf,
}

impl Node {
  fn command(&self, dir: &Path) -> Command {
    let mut command = Command::new(&self.exe);
    command.current_dir(dir);
    // so anything Node starts itself finds the same Node
    if let Some(bin) = self.exe.parent() {
      let path = std::env::var_os("PATH").unwrap_or_default();
      let paths = std::iter::once(bin.to_path_buf()).chain(std::env::split_paths(&path));
      if let Ok(joined) = std::env::join_paths(paths) {
        command.env("PATH", joined);
      }
    }
    command
  }

  /// npm run by this Node, rather than through its shell shim
  fn npm(&self, dir: &Path) -> Command {
    let bin = self.exe.parent().unwrap_or(Path::new("."));
    // Windows installs keep npm beside node; Unix ones under lib/
    let cli = [
      bin.join("node_modules/npm/bin/npm-cli.js"),
      bin.join("../lib/node_modules/npm/bin/npm-cli.js"),
    ]
    .into_iter()
    .find(|cli| cli.exists());
    let mut command = self.command(dir);
    match cli {
      Some(cli) => {
        command.arg(cli);
      }
      None => command = Command::new(if cfg!(windows) { "npm.cmd" } else { "npm" }),
    }
    command.current_dir(dir);
    command
  }
}

/// The Node on the PATH if Vite supports it, else the newest supported one in a usual install location
fn find_node() -> Option<Node> {
  let exe = if cfg!(windows) { "node.exe" } else { "node" };
  let home = std::env::var_os("USERPROFILE")
    .or_else(|| std::env::var_os("HOME"))
    .map(PathBuf::from);

  let mut installed: Vec<PathBuf> = Vec::new();
  // nvm (also nvm in Git Bash on Windows)
  let nvm = std::env::var_os("NVM_DIR")
    .map(PathBuf::from)
    .or_else(|| home.as_ref().map(|home| home.join(".nvm")));
  if let Some(nvm) = nvm {
    installed.extend(versions_in(&nvm.join("versions/node")).map(|dir| dir.join("bin").join(exe)));
  }
  // nvm-windows
  let nvm_windows = std::env::var_os("NVM_HOME")
    .map(PathBuf::from)
    .or_else(|| std::env::var_os("APPDATA").map(|appdata| PathBuf::from(appdata).join("nvm")));
  if let Some(nvm) = nvm_windows {
    installed.extend(versions_in(&nvm).map(|dir| dir.join(exe)));
  }
  if let Some(program_files) = std::env::var_os("ProgramFiles") {
    installed.push(PathBuf::from(program_files).join("nodejs").join(exe));
  }

  let mut candidates: Vec<(PathBuf, (u32, u32, u32))> = installed
    .into_iter()
    .filter(|path| path.exists())
    .filter_map(|path| Some((path.clone(), node_version(&path)?)))
    .collect();
  candidates.sort_by(|a, b| b.1.cmp(&a.1));

  std::iter::once(PathBuf::from("node"))
    .filter_map(|path| Some((path.clone(), node_version(&path)?)))
    .chain(candidates)
    .find(|(_, version)| vite_supports(*version))
    .map(|(exe, _)| Node { exe })
}

/// Version directories (`v24.3.0` and the like) in an install root
fn versions_in(root: &Path) -> impl Iterator<Item = PathBuf> {
  std::fs::read_dir(root)
    .into_iter()
    .flatten()
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.is_dir())
}

fn node_version(exe: &Path) -> Option<(u32, u32, u32)> {
  let output = Command::new(exe).arg("--version").output().ok()?;
  let version = String::from_utf8_lossy(&output.stdout);
  let mut parts = version.trim().trim_start_matches('v').split('.').map(|p| p.parse().ok());
  Some((parts.next()??, parts.next()??, parts.next()??))
}

/// Vite's own requirement: `^20.19.0 || >=22.12.0`
fn vite_supports((major, minor, _): (u32, u32, u32)) -> bool {
  (major == 20 && minor >= 19) || (major == 22 && minor >= 12) || major > 22
}

/// Runs a step of the web build, failing the cargo build with its output if it fails: a web app that doesn't build is
/// as much a build error as Rust that doesn't
fn run(command: &mut Command, step: &str) {
  let output = command
    .output()
    .unwrap_or_else(|err| panic!("web console: couldn't start `{step}`: {err}"));
  if !output.status.success() {
    panic!(
      "web console: `{step}` failed\n{}{}",
      String::from_utf8_lossy(&output.stdout),
      String::from_utf8_lossy(&output.stderr)
    );
  }
}
