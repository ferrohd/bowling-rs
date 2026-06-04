//! Standard ten-pin bowling ruleset.

use crate::geometry::{PinGeometry, TEN_PIN_GEOMETRY};

use super::{BonusScheme, DeadwoodPolicy, Ruleset};

/// Standard ten-pin bowling.
///
/// - 10 pins, 10 frames, 2 balls per frame.
/// - Deadwood is cleared after each delivery.
/// - Strike = 10 + next 2 deliveries; spare = 10 + next 1 delivery.
/// - The 10th frame grants fill balls on a strike or spare.
#[derive(Debug, Clone, Copy)]
pub struct TenPin;

impl Ruleset for TenPin {
    const PIN_COUNT: u8 = 10;
    const FRAME_COUNT: u8 = 10;
    const BALLS_PER_FRAME: u8 = 2;
    const DEADWOOD: DeadwoodPolicy = DeadwoodPolicy::Cleared;
    const BONUS: BonusScheme = BonusScheme::Traditional {
        strike_bonus_balls: 2,
        spare_bonus_balls: 1,
    };
    const ALL_DOWN_IS_SPARE: bool = true;

    fn geometry() -> Option<&'static PinGeometry> {
        Some(&TEN_PIN_GEOMETRY)
    }

    fn name() -> &'static str {
        "Ten-Pin Bowling"
    }
}
