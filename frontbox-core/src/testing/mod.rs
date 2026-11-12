//! Test support for apps and crates built on frontbox, with the `testing` feature. Add it as a dev-dependency:
//!
//! ```toml
//! [dev-dependencies]
//! frontbox-core = { path = "../frontbox-core", features = ["testing"] }
//! ```
//!
//! A [`Scenario`] runs a whole app against virtual hardware, for testing how its systems work together. To test one
//! system's logic on its own, call it with a [`TestContext`](crate::systems::TestContext) instead.

mod scenario;

pub use scenario::*;
