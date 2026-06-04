//! Candlepin bowling ruleset.

use crate::{geometry::PinGeometry, pins::PinSet};

use super::{BonusScheme, DeadwoodPolicy, Ruleset};

/// Candlepin bowling.
///
/// - 10 pins, 10 frames, **3** balls per frame.
/// - Deadwood **remains** on the deck (fallen pins are not cleared).
/// - Strike = 10 + next 2 deliveries; spare = 10 + next 1 delivery.
/// - Clearing all pins with 2 balls is a spare; clearing with 3 is also a
///   spare (all-down-is-spare = true).
/// - No standard geometry for split detection.
#[derive(Debug, Clone, Copy)]
pub struct Candlepin;

impl Ruleset for Candlepin {
    const PIN_COUNT: u8 = 10;
    const FRAME_COUNT: u8 = 10;
    const BALLS_PER_FRAME: u8 = 3;
    const DEADWOOD: DeadwoodPolicy = DeadwoodPolicy::Remains;
    const BONUS: BonusScheme = BonusScheme::Traditional {
        strike_bonus_balls: 2,
        spare_bonus_balls: 1,
    };
    const ALL_DOWN_IS_SPARE: bool = true;

    fn full_rack() -> PinSet {
        PinSet::full::<10>()
    }

    fn geometry() -> Option<&'static PinGeometry> {
        None
    }

    fn name() -> &'static str {
        "Candlepin Bowling"
    }
}
