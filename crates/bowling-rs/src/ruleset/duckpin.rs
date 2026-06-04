//! Duckpin bowling ruleset.

use crate::geometry::PinGeometry;

use super::{BonusScheme, DeadwoodPolicy, Ruleset};

/// Duckpin bowling.
///
/// - 10 pins, 10 frames, **3** balls per frame.
/// - Deadwood is **cleared** after each delivery (like ten-pin).
/// - Strike = 10 + next 2 deliveries; spare (clearing with 2 balls) = 10 +
///   next 1 delivery.
/// - Clearing all pins using all 3 balls is a flat 10 ("ten"), **no bonus**.
/// - No standard geometry for split detection.
#[derive(Debug, Clone, Copy)]
pub struct Duckpin;

impl Ruleset for Duckpin {
    const PIN_COUNT: u8 = 10;
    const FRAME_COUNT: u8 = 10;
    const BALLS_PER_FRAME: u8 = 3;
    const DEADWOOD: DeadwoodPolicy = DeadwoodPolicy::Cleared;
    const BONUS: BonusScheme = BonusScheme::Traditional {
        strike_bonus_balls: 2,
        spare_bonus_balls: 1,
    };
    /// In duckpin, clearing with all 3 balls is a flat 10 (no spare bonus).
    const ALL_DOWN_IS_SPARE: bool = false;

    fn geometry() -> Option<&'static PinGeometry> {
        None
    }

    fn name() -> &'static str {
        "Duckpin Bowling"
    }
}
