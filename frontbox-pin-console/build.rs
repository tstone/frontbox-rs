fn main() {
  // rust-embed decides at compile time whether web/dist exists, so rebuild when the web app does
  println!("cargo:rerun-if-changed=web/dist");
}
