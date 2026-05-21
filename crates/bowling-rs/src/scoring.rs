//! Cumulative scoring engine.
//!
//! Computes per-frame cumulative scores from a sequence of [`ScoredFrame`]s
//! using the bonus rules defined by the [`Ruleset`].

use std::fmt;

use crate::frame::scored::ScoredFrame;
use crate::roll::Roll;
use crate::ruleset::{BonusScheme, Ruleset};

// ---------------------------------------------------------------------------
// Scoreboard
// ---------------------------------------------------------------------------

/// A per-frame score breakdown for a single player's line.
///
/// Frames whose bonus has been fully resolved carry their cumulative score
/// in the [`Resolved`](Self::Resolved) variant. Frames still awaiting
/// future deliveries are [`Pending`](Self::Pending), which structurally
/// cannot carry a bonus or cumulative value, making illegal states
/// unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameScore {
    /// Bonus has been fully resolved; cumulative score is known.
    Resolved {
        /// 1-indexed frame number.
        frame: u8,
        /// Base pin count (before bonuses).
        base: u16,
        /// Bonus pins from subsequent deliveries.
        bonus: u16,
        /// Cumulative score up to and including this frame.
        cumulative: u16,
    },
    /// Awaiting future deliveries to resolve the bonus.
    ///
    /// Only the base pin count is known; bonus and cumulative are not yet
    /// computable.
    Pending {
        /// 1-indexed frame number.
        frame: u8,
        /// Base pin count (before bonuses).
        base: u16,
    },
}

impl FrameScore {
    /// Returns the 1-indexed frame number (present in both variants).
    pub fn frame(&self) -> u8 {
        match self {
            Self::Resolved { frame, .. } | Self::Pending { frame, .. } => *frame,
        }
    }

    /// Returns the base pin count (present in both variants).
    pub fn base(&self) -> u16 {
        match self {
            Self::Resolved { base, .. } | Self::Pending { base, .. } => *base,
        }
    }
}

/// Full scoreboard for a player's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scoreboard {
    /// Per-frame breakdown.
    pub frames: Vec<FrameScore>,
    /// Total score (same as last frame's cumulative, or 0 if no frames).
    pub total: u16,
}

impl fmt::Display for Scoreboard {
    /// Renders a text-based score table.
    ///
    /// ```text
    /// Frame |  1 |  2 |  3 |  4 |  5 |  6 |  7 |  8 |  9 | 10 |
    /// Base  | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 10 | 30 |
    /// Bonus | 20 | 20 | 20 | 20 | 17 | 13 | 10 |  7 |  0 |  0 |
    /// Cum.  | 30 | 60 | 90 |120 |147 |170 |190 |207 |217 |247 |
    /// Total: 247
    /// ```
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.frames.is_empty() {
            return write!(f, "No frames recorded. Total: {}", self.total);
        }

        // Determine column width: at least 4 to fit header labels
        let col_width = 4;

        // Header row
        write!(f, "Frame |")?;
        for fs in &self.frames {
            write!(f, "{:>w$} |", fs.frame(), w = col_width)?;
        }
        writeln!(f)?;

        // Base row
        write!(f, "Base  |")?;
        for fs in &self.frames {
            write!(f, "{:>w$} |", fs.base(), w = col_width)?;
        }
        writeln!(f)?;

        // Bonus row
        write!(f, "Bonus |")?;
        for fs in &self.frames {
            match fs {
                FrameScore::Resolved { bonus, .. } => {
                    write!(f, "{bonus:>col_width$} |")?;
                }
                FrameScore::Pending { .. } => {
                    write!(f, "{:>w$} |", "?", w = col_width)?;
                }
            }
        }
        writeln!(f)?;

        // Cumulative row
        write!(f, "Cum.  |")?;
        for fs in &self.frames {
            match fs {
                FrameScore::Resolved { cumulative, .. } => {
                    write!(f, "{cumulative:>col_width$} |")?;
                }
                FrameScore::Pending { .. } => {
                    write!(f, "{:>w$} |", "?", w = col_width)?;
                }
            }
        }
        writeln!(f)?;

        write!(f, "Total: {}", self.total)
    }
}

// ---------------------------------------------------------------------------
// Scoring logic
// ---------------------------------------------------------------------------

/// Compute the scoreboard for a completed (or in-progress) line.
///
/// The function processes as many frames as have enough subsequent rolls to
/// resolve bonuses. Frames that don't yet have enough future rolls are
/// excluded from the cumulative total (their score is pending).
pub fn compute_scoreboard<R: Ruleset>(frames: &[ScoredFrame]) -> Scoreboard {
    // Flatten all rolls across all frames into a single sequence for
    // lookahead bonus calculation.
    let all_rolls: Vec<&Roll> = frames.iter().flat_map(|f| f.rolls().iter()).collect();

    let BonusScheme::Traditional {
        strike_bonus_balls,
        spare_bonus_balls,
    } = R::BONUS;

    let mut frame_scores = Vec::with_capacity(frames.len());
    let mut cumulative: u16 = 0;
    let mut roll_offset: usize = 0;

    for frame in frames {
        let frame_roll_count = frame.rolls().len();
        let bonus_needed = frame.bonus_balls(strike_bonus_balls, spare_bonus_balls);

        // Collect bonus pins from rolls *after* this frame's rolls.
        let bonus_start = roll_offset + frame_roll_count;
        let bonus: u16 = all_rolls
            .get(bonus_start..)
            .unwrap_or_default()
            .iter()
            .take(bonus_needed as usize)
            .map(|r| u16::from(r.score()))
            .sum();

        // Only include in cumulative if we have enough bonus rolls
        // (or no bonus is needed).
        let available_bonus = all_rolls.len().saturating_sub(bonus_start);
        let resolved = available_bonus >= bonus_needed as usize;

        let base = frame.base_score();

        if resolved {
            cumulative += base + bonus;
            frame_scores.push(FrameScore::Resolved {
                frame: frame.number(),
                base,
                bonus,
                cumulative,
            });
        } else {
            // Not resolved; can't compute further cumulative scores
            // (subsequent frames may also depend on unresolved bonuses).
            // Record only the base for display.
            frame_scores.push(FrameScore::Pending {
                frame: frame.number(),
                base,
            });
        }

        roll_offset += frame_roll_count;
    }

    let total = frame_scores
        .iter()
        .rev()
        .find_map(|fs| match fs {
            FrameScore::Resolved { cumulative, .. } => Some(*cumulative),
            FrameScore::Pending { .. } => None,
        })
        .unwrap_or(0);

    Scoreboard {
        frames: frame_scores,
        total,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FrameKind;
    use crate::pins::PinSet;
    use crate::roll::Roll;
    use crate::ruleset::TenPin;

    fn strike_frame(number: u8) -> ScoredFrame {
        ScoredFrame::new(
            number,
            false,
            FrameKind::Strike,
            vec![Roll::clean(PinSet::full(10).unwrap())],
            10,
        )
    }

    fn spare_frame(number: u8, first: u8) -> ScoredFrame {
        let second = 10 - first;
        let first_pins = PinSet::from_raw((1u16 << first) - 1);
        let second_pins = PinSet::from_raw(((1u16 << 10) - 1) & !((1u16 << first) - 1));
        ScoredFrame::new(
            number,
            false,
            FrameKind::Spare,
            vec![Roll::clean(first_pins), Roll::clean(second_pins)],
            u16::from(first + second),
        )
    }

    fn open_frame(number: u8, first: u8, second: u8) -> ScoredFrame {
        let first_pins = PinSet::from_raw((1u16 << first) - 1);
        let second_pins =
            PinSet::from_raw(((1u16 << (first + second)) - 1) & !((1u16 << first) - 1));
        ScoredFrame::new(
            number,
            false,
            FrameKind::Open,
            vec![Roll::clean(first_pins), Roll::clean(second_pins)],
            u16::from(first + second),
        )
    }

    #[test]
    fn perfect_game_scoring() {
        // 12 strikes: frames 1-9 are regular strikes, frame 10 is final
        let mut frames: Vec<ScoredFrame> = (1..=9).map(strike_frame).collect();
        // Final frame: 3 strikes
        frames.push(ScoredFrame::new(
            10,
            true,
            FrameKind::Strike,
            vec![
                Roll::clean(PinSet::full(10).unwrap()),
                Roll::clean(PinSet::full(10).unwrap()),
                Roll::clean(PinSet::full(10).unwrap()),
            ],
            30,
        ));

        let sb = compute_scoreboard::<TenPin>(&frames);
        assert_eq!(sb.total, 300);
    }

    #[test]
    fn all_gutter_scoring() {
        let mut frames: Vec<ScoredFrame> = (1..=9)
            .map(|n| open_frame(n, 0, 0))
            .collect();
        frames.push(ScoredFrame::new(
            10,
            true,
            FrameKind::Open,
            vec![
                Roll::clean(PinSet::EMPTY),
                Roll::clean(PinSet::EMPTY),
            ],
            0,
        ));
        let sb = compute_scoreboard::<TenPin>(&frames);
        assert_eq!(sb.total, 0);
    }

    #[test]
    fn all_fives_spare_scoring() {
        // Each frame: 5 + spare, next ball 5 → frame score = 15
        // Last frame: 5, spare, 5 → base 15, no bonus (final)
        let mut frames: Vec<ScoredFrame> = (1..=9)
            .map(|n| spare_frame(n, 5))
            .collect();
        frames.push(ScoredFrame::new(
            10,
            true,
            FrameKind::Spare,
            vec![
                Roll::clean(PinSet::from_raw(0b0001_1111)),
                Roll::clean(PinSet::from_raw(0b0011_1110_0000)),
                Roll::clean(PinSet::from_raw(0b0001_1111)),
            ],
            15,
        ));
        let sb = compute_scoreboard::<TenPin>(&frames);
        assert_eq!(sb.total, 150);
    }
}
