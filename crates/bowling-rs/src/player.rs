//! Player identity.
//!
//! A [`Player`] is just a name (and in the future,
//! possibly a handicap or league ID). Per-game state (frames, scores) lives
//! in [`ScoreCard`](crate::scorecard::ScoreCard), and the association is
//! managed by the [`Game`](crate::game::Game).

use crate::error::BowlingError;

/// A player's identity.
///
/// Construction validates that the name is non-empty, making it structurally
/// impossible to create a player with a blank name regardless of the
/// construction path.
#[derive(Debug, Clone)]
pub struct Player {
    /// The player's display name (guaranteed non-empty).
    name: String,
}

impl Player {
    /// Creates a new player with the given name.
    ///
    /// # Errors
    ///
    /// Returns [`BowlingError::EmptyPlayerName`] if `name` is empty.
    pub fn new(name: impl Into<String>) -> Result<Self, BowlingError> {
        let name = name.into();
        if name.is_empty() {
            return Err(BowlingError::EmptyPlayerName);
        }
        Ok(Self { name })
    }

    /// Returns the player's name.
    pub fn name(&self) -> &str {
        &self.name
    }
}
