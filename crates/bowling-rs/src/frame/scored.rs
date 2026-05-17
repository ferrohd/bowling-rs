//! Completed frame representation.

use crate::roll::Roll;

// ---------------------------------------------------------------------------
// FrameKind
// ---------------------------------------------------------------------------

/// The classification of a completed frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// First-ball clearance of all pins.
    Strike,
    /// All pins cleared within the allowed balls (but not the first ball).
    /// For duckpin, clearing with all 3 balls is [`FrameKind::AllDown`], not
    /// a spare.
    Spare,
    /// All pins cleared using all allowed balls in a variant where this does
    /// **not** earn a spare bonus (duckpin "ten").
    AllDown,
    /// Not all pins were knocked down.
    Open,
}

// ---------------------------------------------------------------------------
// ScoredFrame
// ---------------------------------------------------------------------------

/// A completed frame with its deliveries recorded.
///
/// The cumulative score is computed separately by the scoring engine, which
/// needs lookahead into future frames for bonus calculations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoredFrame {
    /// The frame number (1-indexed).
    number: u8,
    /// Whether this was the final frame.
    is_final: bool,
    /// The kind of result.
    kind: FrameKind,
    /// All deliveries in the frame (1–3 for regular, up to 3 for final).
    rolls: Vec<Roll>,
    /// Base pin count for this frame (sum of roll scores, before bonuses).
    base_score: u16,
}

impl ScoredFrame {
    /// Creates a new completed frame.
    pub(crate) fn new(
        number: u8,
        is_final: bool,
        kind: FrameKind,
        rolls: Vec<Roll>,
        base_score: u16,
    ) -> Self {
        Self {
            number,
            is_final,
            kind,
            rolls,
            base_score,
        }
    }

    /// Returns the frame number (1-indexed).
    pub fn number(&self) -> u8 {
        self.number
    }

    /// Returns `true` if this was the final frame of the game.
    pub fn is_final(&self) -> bool {
        self.is_final
    }

    /// Returns the classification of this frame (strike, spare, open, etc.).
    pub fn kind(&self) -> FrameKind {
        self.kind
    }

    /// Returns the deliveries recorded in this frame.
    pub fn rolls(&self) -> &[Roll] {
        &self.rolls
    }

    /// Returns the base pin count for this frame (before bonuses).
    pub fn base_score(&self) -> u16 {
        self.base_score
    }

    /// How many future bonus deliveries this frame earns.
    pub fn bonus_balls(&self, strike_bonus: u8, spare_bonus: u8) -> u8 {
        match self.kind {
            FrameKind::Strike => {
                if self.is_final {
                    0
                } else {
                    strike_bonus
                }
            }
            FrameKind::Spare => {
                if self.is_final {
                    0
                } else {
                    spare_bonus
                }
            }
            FrameKind::AllDown | FrameKind::Open => 0,
        }
    }
}
