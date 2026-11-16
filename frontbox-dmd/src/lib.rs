//! Dot matrix displays, as [`DisplayDevice`](frontbox_display::DisplayDevice)s for the `DisplaySystem`. They're kept
//! apart from `frontbox-display` because they need USB, which a game without one shouldn't have to build.

mod pin2dmd;

pub use pin2dmd::*;
