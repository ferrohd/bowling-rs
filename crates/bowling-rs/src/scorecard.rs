//! Per-game score card for one player's line.
//!
//! A [`ScoreCard`] owns the completed [`ScoredFrame`]s for one competitor.
//! It is intentionally **identity-free** and **ruleset-free**: it stores only
//! raw frame data. Cumulative scoring is done by the [`scoring`] module (or
//! via [`Game::scoreboard`](crate::game::Game::scoreboard)), which knows the
//! ruleset.
//!
//! [`scoring`]: crate::scoring

use crate::frame::scored::ScoredFrame;

/// A single competitor's completed frames for one game.
#[derive(Debug, Clone)]
pub struct ScoreCard {
    /// Completed frames in play order.
    frames: Vec<ScoredFrame>,
}

impl ScoreCard {
    /// Creates an empty score card.
    pub(crate) fn new() -> Self {
        Self {
            frames: Vec::with_capacity(12),
        }
    }

    /// Returns the completed frames.
    pub fn frames(&self) -> &[ScoredFrame] {
        &self.frames
    }

    /// Records a completed frame.
    pub(crate) fn push_frame(&mut self, frame: ScoredFrame) {
        self.frames.push(frame);
    }

    /// Returns how many frames have been completed.
    pub fn frames_completed(&self) -> u8 {
        #[expect(clippy::cast_possible_truncation, reason = "frame count is always <= 255")]
        {
            self.frames.len() as u8
        }
    }
}
