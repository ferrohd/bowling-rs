//! # bowling
//!
//! A generic, typestate-driven bowling game engine that models the complete
//! mechanics of bowling: multiple players, strikes, spares, fill balls,
//! split detection, and per-pin deck tracking.
//!
//! ## Design highlights
//!
//! - **Generic over ruleset** via static dispatch. The [`Ruleset`] trait
//!   defines pin count, frame count, deadwood policy, bonus scheme, and
//!   optional pin geometry. Three variants ship out of the box:
//!   [`TenPin`], [`Candlepin`], and [`Duckpin`].
//!
//! - **Typestate state machines** at both the frame and game level. Illegal
//!   transitions (e.g. rolling after the game is complete) are compile
//!   errors, not runtime panics.
//!
//! - **Per-pin deck** via [`PinSet`], a compact `u16` bitset. Split
//!   detection is powered by [`PinGeometry`] adjacency graphs, supplied
//!   per-ruleset.
//!
//! ## Quick start
//!
//! ```
//! use bowling::prelude::*;
//!
//! // Build a single-player ten-pin game
//! let alice = Player::new("Alice").unwrap();
//! let mut progress = GameBuilder::<TenPin>::new(alice).build();
//!
//! // Bowl a perfect game (12 strikes)
//! for _ in 0..12 {
//!     match progress {
//!         Progress::AwaitingRoll(g) => {
//!             progress = g.roll_count(10).unwrap();
//!         }
//!         Progress::Complete(_) => break,
//!     }
//! }
//!
//! if let Progress::Complete(game) = progress {
//!     assert_eq!(game.scoreboard(0).total, 300);
//! }
//! ```
//!
//! [`TenPin`]: ruleset::TenPin
//! [`Candlepin`]: ruleset::Candlepin
//! [`Duckpin`]: ruleset::Duckpin
//! [`PinSet`]: pins::PinSet
//! [`PinGeometry`]: geometry::PinGeometry
//! [`Ruleset`]: ruleset::Ruleset

pub mod error;
pub mod frame;
pub mod game;
pub mod geometry;
pub mod pins;
pub mod player;
pub mod roll;
pub mod ruleset;
pub mod scorecard;
pub mod scoring;

/// Convenience re-exports for the most commonly used types.
pub mod prelude {
    pub use crate::error::BowlingError;
    pub use crate::frame::{FrameKind, FrameNumber, FramePosition, ScoredFrame};
    pub use crate::game::{
        AwaitingRoll, Competitor, Complete, Game, GameBuilder, GamePhase, Progress,
    };
    pub use crate::geometry::{PinGeometry, TEN_PIN_GEOMETRY};
    pub use crate::pins::PinSet;
    pub use crate::player::Player;
    pub use crate::roll::{FoulStatus, Roll};
    pub use crate::ruleset::{BonusScheme, Candlepin, DeadwoodPolicy, Duckpin, Ruleset, TenPin};
    pub use crate::scorecard::ScoreCard;
    pub use crate::scoring::Scoreboard;
}
