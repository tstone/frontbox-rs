/// Adds a player to the current game, or starts the game if one is not in progress
pub struct AddPlayer;

/// Give points to the current player
pub struct AddPoints(pub u32);
/// Set points multiplier for the current player (e.g. "2x playfield")
pub struct SetMultiplier(pub f32);

/// This could be to the next player or to the next "ball" for the current player. If no turns remain, the game is ended.
pub struct AdvanceTurn;
