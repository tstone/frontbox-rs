pub struct StartGame;
pub struct EndGame;

/// Give points to the current player
pub struct AddPoints(pub u32);
/// Set points multiplier for the current player (e.g. "2x playfield")
pub struct SetMultiplier(pub f32);

/// This could be to the next player or to the next "ball" for the current player.
pub struct AdvanceTurn;
