//! A single delivery (roll) in a bowling game.

use crate::pins::PinSet;

/// Represents a single ball delivery.
///
/// Tracks which pins were knocked down and whether a foul occurred. When a
/// foul happens the knocked-down pins are recorded but score zero for that
/// delivery (ruleset-dependent behaviour for deadwood handling).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Roll {
    /// The set of pins knocked down by this delivery.
    knocked: PinSet,
    /// Whether the bowler committed a foul (crossed the line).
    foul: bool,
}

impl Roll {
    /// Creates a new roll.
    #[inline]
    pub const fn new(knocked: PinSet, foul: bool) -> Self {
        Self { knocked, foul }
    }

    /// Convenience: a clean (non-foul) delivery knocking the given pins.
    #[inline]
    pub const fn clean(knocked: PinSet) -> Self {
        Self {
            knocked,
            foul: false,
        }
    }

    /// Returns the set of pins knocked down by this delivery.
    #[inline]
    pub const fn knocked(self) -> PinSet {
        self.knocked
    }

    /// Returns `true` if the bowler committed a foul (crossed the line).
    #[inline]
    pub const fn is_foul(self) -> bool {
        self.foul
    }

    /// The number of pins knocked down. If a foul, still records the physical
    /// result. Scoring zeroes the count separately.
    #[inline]
    pub const fn pin_count(self) -> u8 {
        self.knocked.count()
    }

    /// The score contribution of this delivery: 0 on a foul, otherwise the
    /// count of knocked pins.
    #[inline]
    pub const fn score(self) -> u8 {
        if self.foul {
            0
        } else {
            self.knocked.count()
        }
    }
}
