//! Error types for the bowling engine.

use crate::pins::PinSet;

/// All errors that can occur during a bowling game.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BowlingError {
    /// Attempted to knock down pins that are not currently standing.
    #[error("invalid delivery: tried to knock {knocked} but only {standing} are standing")]
    InvalidDelivery {
        /// The pins the player tried to knock down.
        knocked: PinSet,
        /// The pins currently standing.
        standing: PinSet,
    },

    /// A player name was empty.
    #[error("player name must not be empty")]
    EmptyPlayerName,
}
