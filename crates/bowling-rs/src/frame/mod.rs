//! Frame-level typestate machines for regular and final frames.
//!
//! Each frame is a state machine that consumes deliveries and produces a
//! [`ScoredFrame`] when complete. Regular and final frames are distinct
//! types because the 10th-frame fill-ball rules differ fundamentally.
//!
//! Phase types carry data: each phase struct owns the in-flight
//! state (`standing` pins, accumulated `rolls`, and for [`FinalFillThree`],
//! `first_ball_strike`).
//!
//! Each phase's `roll()` returns a **precise outcome type** whose variants
//! cover exactly the reachable next states. This makes
//! every match exhaustive at compile time and eliminates all `unreachable!`
//! arms.

pub mod scored;

use std::marker::PhantomData;

use crate::error::BowlingError;
use crate::pins::PinSet;
use crate::roll::Roll;
use crate::ruleset::Ruleset;

pub use scored::{FrameKind, ScoredFrame};

// =========================================================================
// FramePhase trait
// =========================================================================

/// Common read access to the state carried by an in-flight frame phase.
///
/// Every phase type implements this trait, giving uniform access to the pins
/// currently standing and the deliveries recorded so far.
pub trait FramePhase: std::fmt::Debug {
    /// The pins currently standing.
    fn standing(&self) -> PinSet;

    /// The deliveries recorded so far in this frame.
    fn rolls(&self) -> &[Roll];
}

// =========================================================================
// Data-carrying phase types
// =========================================================================

/// Awaiting the very first delivery of a frame.
///
/// No rolls have been recorded yet; the full rack is standing.
#[derive(Debug)]
pub struct BallOne {
    standing: PinSet,
}

impl FramePhase for BallOne {
    fn standing(&self) -> PinSet {
        self.standing
    }

    fn rolls(&self) -> &[Roll] {
        &[]
    }
}

/// Awaiting the second delivery of a frame.
#[derive(Debug)]
pub struct BallTwo {
    standing: PinSet,
    rolls: Vec<Roll>,
}

impl FramePhase for BallTwo {
    fn standing(&self) -> PinSet {
        self.standing
    }

    fn rolls(&self) -> &[Roll] {
        &self.rolls
    }
}

/// Awaiting the third delivery (3-ball variants only).
#[derive(Debug)]
pub struct BallThree {
    standing: PinSet,
    rolls: Vec<Roll>,
}

impl FramePhase for BallThree {
    fn standing(&self) -> PinSet {
        self.standing
    }

    fn rolls(&self) -> &[Roll] {
        &self.rolls
    }
}

// Final-frame specific phases

/// Final frame: awaiting fill ball 2 after a strike on ball one.
#[derive(Debug)]
pub struct FinalFillTwo {
    standing: PinSet,
    rolls: Vec<Roll>,
}

impl FramePhase for FinalFillTwo {
    fn standing(&self) -> PinSet {
        self.standing
    }

    fn rolls(&self) -> &[Roll] {
        &self.rolls
    }
}

/// Final frame: awaiting the last fill ball (after strike+strike, or spare).
///
/// This is the **only** phase that carries `first_ball_strike`, since it is
/// the only phase where the distinction matters (to label the final
/// [`ScoredFrame`] as [`FrameKind::Strike`] vs [`FrameKind::Spare`]).
#[derive(Debug)]
pub struct FinalFillThree {
    standing: PinSet,
    rolls: Vec<Roll>,
    /// `true` if the first ball of this final frame was a strike.
    first_ball_strike: bool,
}

impl FramePhase for FinalFillThree {
    fn standing(&self) -> PinSet {
        self.standing
    }

    fn rolls(&self) -> &[Roll] {
        &self.rolls
    }
}

// =========================================================================
// Regular frame
// =========================================================================

/// A regular (non-final) frame, parameterised by ruleset `R` and phase `P`.
///
/// The phase (`BallOne`, `BallTwo`, `BallThree`) carries the in-flight state
/// and determines which deliveries are valid. Transitions consume `self` and
/// return a precise outcome type whose variants cover exactly the reachable
/// next states.
#[derive(Debug)]
pub struct RegularFrame<R: Ruleset, P: FramePhase> {
    /// 1-indexed frame number.
    number: u8,
    /// The current phase, carrying standing pins and accumulated rolls.
    phase: P,
    _ruleset: PhantomData<R>,
}

// --- Precise outcome types for regular frames ---

/// Outcome of the first ball in a regular frame.
///
/// A `BallOne` delivery either advances to [`BallTwo`] or completes the
/// frame (strike).
#[derive(Debug)]
pub enum RegAfterOne<R: Ruleset> {
    /// The frame continues to ball two.
    Continue(RegularFrame<R, BallTwo>),
    /// The frame is complete (strike).
    Done(ScoredFrame),
}

/// Outcome of the second ball in a regular frame.
///
/// A `BallTwo` delivery either advances to [`BallThree`] (3-ball variants)
/// or completes the frame (spare or open).
#[derive(Debug)]
pub enum RegAfterTwo<R: Ruleset> {
    /// The frame continues to ball three (3-ball variants only).
    Continue(RegularFrame<R, BallThree>),
    /// The frame is complete (spare or open).
    Done(ScoredFrame),
}

// `BallThree::roll` is terminal; it returns `Result<ScoredFrame, _>` directly.

impl<R: Ruleset, P: FramePhase> RegularFrame<R, P> {
    /// Returns the pins currently standing in this frame.
    pub fn standing(&self) -> PinSet {
        self.phase.standing()
    }

    /// Returns a reference to the current phase.
    pub fn phase(&self) -> &P {
        &self.phase
    }
}

// Final frame
// =========================================================================

/// The final frame of a game.
///
/// The final frame has special fill-ball rules: on a strike, the bowler gets
/// two more deliveries; on a spare, one more. The pins are reset after each
/// clearance. This type has its own typestate chain, separate from
/// [`RegularFrame`].
#[derive(Debug)]
pub struct FinalFrame<R: Ruleset, P: FramePhase> {
    /// 1-indexed frame number (== `FRAME_COUNT`).
    number: u8,
    /// The current phase, carrying standing pins, accumulated rolls, and
    /// (for [`FinalFillThree`] only) the `first_ball_strike` flag.
    phase: P,
    _ruleset: PhantomData<R>,
}

// --- Precise outcome types for final frames ---

/// Outcome of ball one in the final frame.
#[derive(Debug)]
pub enum FinalAfterOne<R: Ruleset> {
    /// No strike, awaiting ball two.
    ToBallTwo(FinalFrame<R, BallTwo>),
    /// Strike on ball one. Pins reset, awaiting fill ball 2.
    ToFillTwo(FinalFrame<R, FinalFillTwo>),
}

/// Outcome of ball two in the final frame.
#[derive(Debug)]
pub enum FinalAfterTwo<R: Ruleset> {
    /// 3-ball variant, no clearance yet. Awaiting ball three.
    ToBallThree(FinalFrame<R, BallThree>),
    /// Spare (or 3-ball clearance). Pins reset, awaiting fill ball 3.
    ToFillThree(FinalFrame<R, FinalFillThree>),
    /// 2-ball variant, no clearance. Frame is done (open).
    Done(ScoredFrame),
}

/// Outcome of ball three in the final frame.
#[derive(Debug)]
pub enum FinalAfterThree<R: Ruleset> {
    /// All pins cleared using all 3 balls (spare in candlepin). Earns a
    /// fill ball.
    ToFillThree(FinalFrame<R, FinalFillThree>),
    /// Frame is complete (open or all-down without bonus).
    Done(ScoredFrame),
}

// `FinalFillTwo::roll` always transitions to `FinalFillThree` (single
// outcome, returned directly).
// `FinalFillThree::roll` is terminal, returns `ScoredFrame` directly.

impl<R: Ruleset, P: FramePhase> FinalFrame<R, P> {
    /// Returns the pins currently standing in this frame.
    pub fn standing(&self) -> PinSet {
        self.phase.standing()
    }

    /// Returns a reference to the current phase.
    pub fn phase(&self) -> &P {
        &self.phase
    }
}

