//! Completed frame representation.

use super::FrameNumber;
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
// FramePosition
// ---------------------------------------------------------------------------

/// Whether a frame is a regular (non-final) frame or the final frame.
///
/// Replaces a bare `bool` with a self-documenting enum so that call sites
/// read `FramePosition::Final` instead of `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FramePosition {
    /// A regular (non-final) frame.
    Regular,
    /// The final frame of the game (with fill-ball rules).
    Final,
}

impl FramePosition {
    /// Returns `true` if this is the final frame.
    #[inline]
    pub const fn is_final(self) -> bool {
        matches!(self, Self::Final)
    }
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
    number: FrameNumber,
    /// Whether this is a regular or final frame.
    position: FramePosition,
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
        number: FrameNumber,
        position: FramePosition,
        kind: FrameKind,
        rolls: Vec<Roll>,
        base_score: u16,
    ) -> Self {
        Self {
            number,
            position,
            kind,
            rolls,
            base_score,
        }
    }

    /// Returns the frame number (1-indexed).
    pub fn number(&self) -> FrameNumber {
        self.number
    }

    /// Returns the position of this frame (regular or final).
    pub fn position(&self) -> FramePosition {
        self.position
    }

    /// Returns `true` if this was the final frame of the game.
    pub fn is_final(&self) -> bool {
        self.position.is_final()
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
                if self.position.is_final() {
                    0
                } else {
                    strike_bonus
                }
            }
            FrameKind::Spare => {
                if self.position.is_final() {
                    0
                } else {
                    spare_bonus
                }
            }
            FrameKind::AllDown | FrameKind::Open => 0,
        }
    }
}
