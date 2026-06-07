//! A single delivery (roll) in a bowling game.

use crate::pins::PinSet;

// ---------------------------------------------------------------------------
// FoulStatus
// ---------------------------------------------------------------------------

/// Whether a foul was committed on a delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FoulStatus {
    /// The delivery was clean (no foul).
    Clean,
    /// The bowler committed a foul (crossed the line).
    Foul,
}

impl FoulStatus {
    /// Returns `true` if this is a foul.
    #[inline]
    pub const fn is_foul(self) -> bool {
        matches!(self, Self::Foul)
    }

    /// Returns `true` if this is a clean delivery.
    #[inline]
    pub const fn is_clean(self) -> bool {
        matches!(self, Self::Clean)
    }
}

impl From<bool> for FoulStatus {
    fn from(foul: bool) -> Self {
        if foul { Self::Foul } else { Self::Clean }
    }
}

impl From<FoulStatus> for bool {
    fn from(status: FoulStatus) -> Self {
        status.is_foul()
    }
}

// ---------------------------------------------------------------------------
// Roll
// ---------------------------------------------------------------------------

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
    foul: FoulStatus,
}

impl Roll {
    /// Creates a new roll.
    #[inline]
    pub const fn new(knocked: PinSet, foul: FoulStatus) -> Self {
        Self { knocked, foul }
    }

    /// Convenience: a clean (non-foul) delivery knocking the given pins.
    #[inline]
    pub const fn clean(knocked: PinSet) -> Self {
        Self {
            knocked,
            foul: FoulStatus::Clean,
        }
    }

    /// Convenience: a foul delivery knocking the given pins.
    #[inline]
    pub const fn foul(knocked: PinSet) -> Self {
        Self {
            knocked,
            foul: FoulStatus::Foul,
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
        self.foul.is_foul()
    }

    /// Returns the foul status of this delivery.
    #[inline]
    pub const fn foul_status(self) -> FoulStatus {
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
        if self.foul.is_foul() {
            0
        } else {
            self.knocked.count()
        }
    }
}
