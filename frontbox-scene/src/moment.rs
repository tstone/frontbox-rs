use frontbox_gfx::scene::Params;

/// What's the same for every layer of a scene at one moment of playing it. A [`Playback`](crate::Playback) gives its
/// own with [`Playback::moment`](crate::Playback::moment), and [`Scene::frame`](crate::Scene::frame) turns it into what a
/// layer is sampled with.
#[derive(Debug, Clone, Copy)]
pub struct Moment<'a> {
  /// Seconds since the scene started
  pub time: f32,
  /// Seed for random props, new each time the scene restarts so it doesn't repeat the same picks
  pub playback_seed: u32,
  /// Values from the game, overriding the scene's param defaults
  pub params: &'a Params,
}
