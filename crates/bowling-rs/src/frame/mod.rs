//! Frame-level typestate machines for regular and final frames.
//!
//! Each frame is a state machine that consumes deliveries and produces a
//! [`ScoredFrame`] when complete. Regular and final frames are distinct
//! types because the 10th-frame fill-ball rules differ fundamentally.
//!
//! Phase types carry data: each phase struct owns the in-flight
//! state (`standing` pins, accumulated `rolls`, and for [`FinalFillThree`],
//! a [`FirstBallOutcome`]).
//!
//! Each phase's `roll()` returns a **precise outcome type** whose variants
//! cover exactly the reachable next states. This makes
//! every match exhaustive at compile time and eliminates all `unreachable!`
//! arms.

pub mod number;
pub mod scored;

use std::marker::PhantomData;

use crate::error::BowlingError;
use crate::pins::PinSet;
use crate::roll::Roll;
use crate::ruleset::Ruleset;

pub use number::FrameNumber;
pub use scored::{FrameKind, FramePosition, ScoredFrame};

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

/// Whether the first ball of a final frame was a strike.
///
/// Carried only by [`FinalFillThree`] where the distinction matters: it
/// determines whether the completed frame is labelled
/// [`FrameKind::Strike`] or [`FrameKind::Spare`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstBallOutcome {
    /// The first ball of the final frame was a strike.
    Strike,
    /// The first ball of the final frame was not a strike.
    NonStrike,
}

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
/// This is the **only** phase that carries [`FirstBallOutcome`], since it is
/// the only phase where the distinction matters (to label the final
/// [`ScoredFrame`] as [`FrameKind::Strike`] vs [`FrameKind::Spare`]).
#[derive(Debug)]
pub struct FinalFillThree {
    standing: PinSet,
    rolls: Vec<Roll>,
    /// Whether the first ball of this final frame was a strike.
    first_ball_outcome: FirstBallOutcome,
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
    number: FrameNumber,
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

impl<R: Ruleset> RegularFrame<R, BallOne> {
    /// Creates a new regular frame ready for the first delivery.
    pub fn new(number: FrameNumber) -> Result<Self, BowlingError> {
        Ok(Self {
            number,
            phase: BallOne {
                standing: PinSet::try_full(R::PIN_COUNT)?,
            },
            _ruleset: PhantomData,
        })
    }

    /// Deliver the first ball.
    pub fn roll(self, delivery: Roll) -> Result<RegAfterOne<R>, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        let mut rolls = Vec::with_capacity(R::BALLS_PER_FRAME as usize);
        rolls.push(delivery);

        // Strike: all pins down on first ball
        if new_standing.is_empty() {
            return Ok(RegAfterOne::Done(finish_regular(
                self.number,
                rolls,
                FrameKind::Strike,
            )));
        }

        // Advance to ball two
        Ok(RegAfterOne::Continue(RegularFrame {
            number: self.number,
            phase: BallTwo {
                standing: new_standing,
                rolls,
            },
            _ruleset: PhantomData,
        }))
    }
}

impl<R: Ruleset> RegularFrame<R, BallTwo> {
    /// Deliver the second ball.
    pub fn roll(mut self, delivery: Roll) -> Result<RegAfterTwo<R>, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        self.phase.rolls.push(delivery);

        // All pins down on second ball -> spare
        if new_standing.is_empty() {
            return Ok(RegAfterTwo::Done(finish_regular(
                self.number,
                self.phase.rolls,
                FrameKind::Spare,
            )));
        }

        // If this is a 2-ball variant, frame is open
        if R::BALLS_PER_FRAME <= 2 {
            return Ok(RegAfterTwo::Done(finish_regular(
                self.number,
                self.phase.rolls,
                FrameKind::Open,
            )));
        }

        // 3-ball variant: advance to ball three
        Ok(RegAfterTwo::Continue(RegularFrame {
            number: self.number,
            phase: BallThree {
                standing: new_standing,
                rolls: self.phase.rolls,
            },
            _ruleset: PhantomData,
        }))
    }
}

impl<R: Ruleset> RegularFrame<R, BallThree> {
    /// Deliver the third ball (3-ball variants only).
    ///
    /// This is a terminal phase; the frame always completes.
    pub fn roll(mut self, delivery: Roll) -> Result<ScoredFrame, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        self.phase.rolls.push(delivery);

        if new_standing.is_empty() {
            let kind = if R::ALL_DOWN_IS_SPARE {
                FrameKind::Spare
            } else {
                FrameKind::AllDown
            };
            return Ok(finish_regular(self.number, self.phase.rolls, kind));
        }

        Ok(finish_regular(
            self.number,
            self.phase.rolls,
            FrameKind::Open,
        ))
    }
}

// =========================================================================
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
    number: FrameNumber,
    /// The current phase, carrying standing pins, accumulated rolls, and
    /// (for [`FinalFillThree`] only) a [`FirstBallOutcome`].
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

impl<R: Ruleset> FinalFrame<R, BallOne> {
    /// Creates a new final frame.
    pub fn new(number: FrameNumber) -> Result<Self, BowlingError> {
        Ok(Self {
            number,
            phase: BallOne {
                standing: PinSet::try_full(R::PIN_COUNT)?,
            },
            _ruleset: PhantomData,
        })
    }

    /// Deliver ball one of the final frame.
    pub fn roll(self, delivery: Roll) -> Result<FinalAfterOne<R>, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        let mut rolls = Vec::with_capacity(3);
        rolls.push(delivery);

        if new_standing.is_empty() {
            // Strike on first ball -> reset pins, go to fill ball 2
            return Ok(FinalAfterOne::ToFillTwo(FinalFrame {
                number: self.number,
                phase: FinalFillTwo {
                    standing: PinSet::try_full(R::PIN_COUNT)?,
                    rolls,
                },
                _ruleset: PhantomData,
            }));
        }

        // No strike -> ball two
        Ok(FinalAfterOne::ToBallTwo(FinalFrame {
            number: self.number,
            phase: BallTwo {
                standing: new_standing,
                rolls,
            },
            _ruleset: PhantomData,
        }))
    }
}

impl<R: Ruleset> FinalFrame<R, BallTwo> {
    /// Deliver ball two (no strike on ball one).
    pub fn roll(mut self, delivery: Roll) -> Result<FinalAfterTwo<R>, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        self.phase.rolls.push(delivery);

        if new_standing.is_empty() {
            // Spare -> reset pins, fill ball 3
            return Ok(FinalAfterTwo::ToFillThree(FinalFrame {
                number: self.number,
                phase: FinalFillThree {
                    standing: PinSet::try_full(R::PIN_COUNT)?,
                    rolls: self.phase.rolls,
                    first_ball_outcome: FirstBallOutcome::NonStrike,
                },
                _ruleset: PhantomData,
            }));
        }

        // 2-ball variant -> frame done (open)
        if R::BALLS_PER_FRAME <= 2 {
            return Ok(FinalAfterTwo::Done(finish_final(
                self.number,
                self.phase.rolls,
                FrameKind::Open,
            )));
        }

        // 3-ball variant -> ball three
        Ok(FinalAfterTwo::ToBallThree(FinalFrame {
            number: self.number,
            phase: BallThree {
                standing: new_standing,
                rolls: self.phase.rolls,
            },
            _ruleset: PhantomData,
        }))
    }
}

impl<R: Ruleset> FinalFrame<R, BallThree> {
    /// Deliver ball three (3-ball variant, no strike or spare yet).
    pub fn roll(mut self, delivery: Roll) -> Result<FinalAfterThree<R>, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        self.phase.rolls.push(delivery);

        if new_standing.is_empty() {
            if R::ALL_DOWN_IS_SPARE {
                // Spare in the final frame, earns a fill ball
                return Ok(FinalAfterThree::ToFillThree(FinalFrame {
                    number: self.number,
                    phase: FinalFillThree {
                        standing: PinSet::try_full(R::PIN_COUNT)?,
                        rolls: self.phase.rolls,
                        first_ball_outcome: FirstBallOutcome::NonStrike,
                    },
                    _ruleset: PhantomData,
                }));
            }
            return Ok(FinalAfterThree::Done(finish_final(
                self.number,
                self.phase.rolls,
                FrameKind::AllDown,
            )));
        }

        Ok(FinalAfterThree::Done(finish_final(
            self.number,
            self.phase.rolls,
            FrameKind::Open,
        )))
    }
}

impl<R: Ruleset> FinalFrame<R, FinalFillTwo> {
    /// Deliver fill ball 2 (after strike on ball 1).
    ///
    /// Always transitions to [`FinalFillThree`], either with reset pins
    /// (another clearance) or with remaining pins.
    pub fn roll(
        mut self,
        delivery: Roll,
    ) -> Result<FinalFrame<R, FinalFillThree>, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        let new_standing = self.phase.standing - delivery.knocked();
        self.phase.rolls.push(delivery);

        if new_standing.is_empty() {
            // Another clearance -> reset pins
            return Ok(FinalFrame {
                number: self.number,
                phase: FinalFillThree {
                    standing: PinSet::try_full(R::PIN_COUNT)?,
                    rolls: self.phase.rolls,
                    first_ball_outcome: FirstBallOutcome::Strike,
                },
                _ruleset: PhantomData,
            });
        }

        // Not a clearance -> remaining pins
        Ok(FinalFrame {
            number: self.number,
            phase: FinalFillThree {
                standing: new_standing,
                rolls: self.phase.rolls,
                first_ball_outcome: FirstBallOutcome::Strike,
            },
            _ruleset: PhantomData,
        })
    }
}

impl<R: Ruleset> FinalFrame<R, FinalFillThree> {
    /// Deliver the last fill ball.
    ///
    /// This is a terminal phase; the frame always completes.
    pub fn roll(mut self, delivery: Roll) -> Result<ScoredFrame, BowlingError> {
        validate_delivery(self.phase.standing, delivery)?;

        self.phase.rolls.push(delivery);

        let kind = match self.phase.first_ball_outcome {
            FirstBallOutcome::Strike => FrameKind::Strike,
            FirstBallOutcome::NonStrike => FrameKind::Spare,
        };

        Ok(finish_final(self.number, self.phase.rolls, kind))
    }
}

// =========================================================================
// Internal helpers
// =========================================================================

/// Validate that `delivery.knocked()` is a subset of `standing`.
fn validate_delivery(standing: PinSet, delivery: Roll) -> Result<(), BowlingError> {
    if !standing.contains_all(delivery.knocked()) {
        return Err(BowlingError::InvalidDelivery {
            knocked: delivery.knocked(),
            standing,
        });
    }
    Ok(())
}

/// Assemble a `ScoredFrame` for a completed regular frame.
fn finish_regular(number: FrameNumber, rolls: Vec<Roll>, kind: FrameKind) -> ScoredFrame {
    let base_score = rolls.iter().map(|r| u16::from(r.score())).sum();
    ScoredFrame::new(number, FramePosition::Regular, kind, rolls, base_score)
}

/// Assemble a `ScoredFrame` for a completed final frame.
fn finish_final(number: FrameNumber, rolls: Vec<Roll>, kind: FrameKind) -> ScoredFrame {
    let base_score = rolls.iter().map(|r| u16::from(r.score())).sum();
    ScoredFrame::new(number, FramePosition::Final, kind, rolls, base_score)
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruleset::{Candlepin, Duckpin, TenPin};

    fn frame(n: u8) -> FrameNumber {
        FrameNumber::new(n).unwrap()
    }

    fn strike_roll() -> Roll {
        Roll::clean(PinSet::full::<10>())
    }

    fn knock(n: u8) -> Roll {
        if n == 0 {
            Roll::clean(PinSet::EMPTY)
        } else {
            Roll::clean(PinSet::range(0, n))
        }
    }

    #[test]
    fn regular_frame_strike() {
        let frame = RegularFrame::<TenPin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(strike_roll()).unwrap();
        let RegAfterOne::Done(sf) = outcome else {
            panic!("expected Done")
        };
        assert_eq!(sf.kind(), FrameKind::Strike);
        assert_eq!(sf.base_score(), 10);
        assert_eq!(sf.rolls().len(), 1);
    }

    #[test]
    fn regular_frame_spare() {
        let frame = RegularFrame::<TenPin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(knock(7)).unwrap();
        let RegAfterOne::Continue(frame2) = outcome else {
            panic!("expected Continue")
        };
        let remaining = Roll::clean(PinSet::range(7, 10));
        let outcome2 = frame2.roll(remaining).unwrap();
        let RegAfterTwo::Done(sf) = outcome2 else {
            panic!("expected Done")
        };
        assert_eq!(sf.kind(), FrameKind::Spare);
        assert_eq!(sf.base_score(), 10);
    }

    #[test]
    fn regular_frame_open() {
        let frame = RegularFrame::<TenPin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(knock(3)).unwrap();
        let RegAfterOne::Continue(frame2) = outcome else {
            panic!("expected Continue")
        };
        let second = Roll::clean(PinSet::of([3, 4]));
        let outcome2 = frame2.roll(second).unwrap();
        let RegAfterTwo::Done(sf) = outcome2 else {
            panic!("expected Done")
        };
        assert_eq!(sf.kind(), FrameKind::Open);
        assert_eq!(sf.base_score(), 5);
    }

    #[test]
    fn invalid_delivery_rejected() {
        let frame = RegularFrame::<TenPin, BallOne>::new(frame(1)).unwrap();
        let bad = Roll::clean(PinSet::of([15]));
        assert!(frame.roll(bad).is_err());
    }

    #[test]
    fn final_frame_three_strikes() {
        let f = FinalFrame::<TenPin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToFillTwo(f2) = f.roll(strike_roll()).unwrap() else {
            panic!("expected ToFillTwo")
        };
        // FinalFillTwo::roll returns FinalFrame<FinalFillThree> directly
        let f3 = f2.roll(strike_roll()).unwrap();
        // FinalFillThree::roll returns ScoredFrame directly
        let sf = f3.roll(strike_roll()).unwrap();
        assert_eq!(sf.kind(), FrameKind::Strike);
        assert_eq!(sf.base_score(), 30);
        assert_eq!(sf.rolls().len(), 3);
        assert!(sf.is_final());
    }

    #[test]
    fn final_frame_spare_plus_fill() {
        let f = FinalFrame::<TenPin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToBallTwo(f2) = f.roll(knock(7)).unwrap() else {
            panic!("expected ToBallTwo")
        };
        let spare_roll = Roll::clean(PinSet::range(7, 10));
        let FinalAfterTwo::ToFillThree(f3) = f2.roll(spare_roll).unwrap() else {
            panic!("expected ToFillThree")
        };
        // FinalFillThree::roll returns ScoredFrame directly
        let sf = f3.roll(knock(5)).unwrap();
        assert_eq!(sf.kind(), FrameKind::Spare);
        assert_eq!(sf.base_score(), 15);
        assert!(sf.is_final());
    }

    // --- Gutter & boundary edge cases ---

    #[test]
    fn regular_frame_gutter_then_spare() {
        // Ball 1: gutter (0 pins). Ball 2: clears full rack → spare.
        let frame = RegularFrame::<TenPin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(knock(0)).unwrap();
        let RegAfterOne::Continue(frame2) = outcome else {
            panic!("expected Continue after gutter")
        };
        // All 10 pins still standing; knock them all on ball 2
        assert_eq!(frame2.standing().count(), 10);
        let outcome2 = frame2.roll(strike_roll()).unwrap();
        let RegAfterTwo::Done(sf) = outcome2 else {
            panic!("expected Done")
        };
        assert_eq!(sf.kind(), FrameKind::Spare);
        assert_eq!(sf.base_score(), 10);
    }

    #[test]
    fn regular_frame_foul_removes_pins_but_scores_zero() {
        // Ball 1: foul knocking 3 pins. Pins are removed from standing,
        // but the delivery scores 0.
        let foul_delivery = Roll::foul(PinSet::of([0, 1, 2]));
        let frame = RegularFrame::<TenPin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(foul_delivery).unwrap();
        let RegAfterOne::Continue(frame2) = outcome else {
            panic!("expected Continue; foul didn't clear all pins")
        };
        assert_eq!(frame2.standing().count(), 7);

        // Ball 2: clean knock of 4 of the remaining 7.
        // Remaining standing is pins 3..=9; knock pins 3,4,5,6.
        let ball2 = Roll::clean(PinSet::of([3, 4, 5, 6]));
        let outcome2 = frame2.roll(ball2).unwrap();
        let RegAfterTwo::Done(sf) = outcome2 else {
            panic!("expected Done")
        };
        assert_eq!(sf.kind(), FrameKind::Open);
        // base_score = foul(0) + clean(4) = 4
        assert_eq!(sf.base_score(), 4);
    }

    // --- Final frame edge cases ---

    #[test]
    fn final_frame_strike_then_open_fill() {
        // Strike on ball 1 → fill-2 knocks 6 → fill-3 knocks 2 of
        // the remaining 4. Pins reset after strike, NOT after fill-2
        // partial.
        let f = FinalFrame::<TenPin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToFillTwo(f2) = f.roll(strike_roll()).unwrap() else {
            panic!("expected ToFillTwo")
        };
        // Pins reset to 10 after the strike
        assert_eq!(f2.standing().count(), 10);

        let fill2 = knock(6);
        let f3 = f2.roll(fill2).unwrap();
        // Not a clearance, so pins should NOT be reset. 4 remain
        assert_eq!(f3.standing().count(), 4);

        // Knock 2 of the remaining 4 (pins 6, 7)
        let fill3 = Roll::clean(PinSet::of([6, 7]));
        let sf = f3.roll(fill3).unwrap();
        assert_eq!(sf.kind(), FrameKind::Strike);
        assert_eq!(sf.base_score(), 18); // 10 + 6 + 2
        assert!(sf.is_final());
        assert_eq!(sf.rolls().len(), 3);
    }

    #[test]
    fn final_frame_open() {
        // Ball 1: 3. Ball 2: 4. No clearance → done, no fill ball.
        let f = FinalFrame::<TenPin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToBallTwo(f2) = f.roll(knock(3)).unwrap() else {
            panic!("expected ToBallTwo")
        };
        // Pins 0,1,2 are down; knock pins 3,4,5,6 (4 pins from remaining)
        let ball2 = Roll::clean(PinSet::of([3, 4, 5, 6]));
        let FinalAfterTwo::Done(sf) = f2.roll(ball2).unwrap() else {
            panic!("expected Done (open)")
        };
        assert_eq!(sf.kind(), FrameKind::Open);
        assert_eq!(sf.base_score(), 7);
        assert!(sf.is_final());
    }

    #[test]
    fn final_frame_gutter_gutter() {
        let f = FinalFrame::<TenPin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToBallTwo(f2) = f.roll(knock(0)).unwrap() else {
            panic!("expected ToBallTwo")
        };
        let FinalAfterTwo::Done(sf) = f2.roll(knock(0)).unwrap() else {
            panic!("expected Done")
        };
        assert_eq!(sf.kind(), FrameKind::Open);
        assert_eq!(sf.base_score(), 0);
        assert!(sf.is_final());
    }

    // --- 3-ball variant tests (Candlepin / Duckpin) ---

    /// Helper: knock `n` lowest-indexed pins from a given standing set.
    fn knock_n_from(standing: PinSet, n: u8) -> Roll {
        Roll::clean(standing.into_iter().take(n as usize).collect())
    }

    #[test]
    fn candlepin_regular_three_ball_spare() {
        // Candlepin: 3 balls per frame. Knock 3 → 4 → 3 (all 10 down).
        // ALL_DOWN_IS_SPARE = true → FrameKind::Spare
        let frame = RegularFrame::<Candlepin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(knock(3)).unwrap();
        let RegAfterOne::Continue(f2) = outcome else {
            panic!("expected Continue")
        };
        assert_eq!(f2.standing().count(), 7);

        let ball2 = knock_n_from(f2.standing(), 4);
        let outcome2 = f2.roll(ball2).unwrap();
        let RegAfterTwo::Continue(f3) = outcome2 else {
            panic!("expected Continue to BallThree")
        };
        assert_eq!(f3.standing().count(), 3);

        let ball3 = knock_n_from(f3.standing(), 3);
        let sf = f3.roll(ball3).unwrap();
        assert_eq!(sf.kind(), FrameKind::Spare);
        assert_eq!(sf.base_score(), 10);
        assert!(!sf.is_final());
    }

    #[test]
    fn duckpin_regular_three_ball_alldown() {
        // Duckpin: same rolls as above, 3 → 4 → 3 (all 10 down).
        // ALL_DOWN_IS_SPARE = false → FrameKind::AllDown (no bonus)
        let frame = RegularFrame::<Duckpin, BallOne>::new(frame(1)).unwrap();
        let outcome = frame.roll(knock(3)).unwrap();
        let RegAfterOne::Continue(f2) = outcome else {
            panic!("expected Continue")
        };

        let ball2 = knock_n_from(f2.standing(), 4);
        let outcome2 = f2.roll(ball2).unwrap();
        let RegAfterTwo::Continue(f3) = outcome2 else {
            panic!("expected Continue to BallThree")
        };

        let ball3 = knock_n_from(f3.standing(), 3);
        let sf = f3.roll(ball3).unwrap();
        assert_eq!(sf.kind(), FrameKind::AllDown);
        assert_eq!(sf.base_score(), 10);
        assert!(!sf.is_final());
    }

    #[test]
    fn candlepin_final_three_ball_spare_earns_fill() {
        // Candlepin final frame: 3 → 4 → 3 (all down via 3 balls).
        // ALL_DOWN_IS_SPARE = true → earns a fill ball.
        let f = FinalFrame::<Candlepin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToBallTwo(f2) = f.roll(knock(3)).unwrap() else {
            panic!("expected ToBallTwo")
        };

        let ball2 = knock_n_from(f2.standing(), 4);
        let FinalAfterTwo::ToBallThree(f3) = f2.roll(ball2).unwrap() else {
            panic!("expected ToBallThree (3-ball variant)")
        };

        let ball3 = knock_n_from(f3.standing(), 3);
        let FinalAfterThree::ToFillThree(fill) = f3.roll(ball3).unwrap() else {
            panic!("expected ToFillThree; candlepin 3-ball spare earns fill")
        };
        // Pins reset after spare
        assert_eq!(fill.standing().count(), 10);

        let sf = fill.roll(knock(6)).unwrap();
        assert_eq!(sf.kind(), FrameKind::Spare);
        assert_eq!(sf.base_score(), 16); // 3+4+3+6
        assert!(sf.is_final());
    }

    #[test]
    fn duckpin_final_three_ball_alldown_no_fill() {
        // Duckpin final frame: 3 → 4 → 3 (all down via 3 balls).
        // ALL_DOWN_IS_SPARE = false → Done immediately, no fill ball.
        let f = FinalFrame::<Duckpin, BallOne>::new(frame(10)).unwrap();
        let FinalAfterOne::ToBallTwo(f2) = f.roll(knock(3)).unwrap() else {
            panic!("expected ToBallTwo")
        };

        let ball2 = knock_n_from(f2.standing(), 4);
        let FinalAfterTwo::ToBallThree(f3) = f2.roll(ball2).unwrap() else {
            panic!("expected ToBallThree (3-ball variant)")
        };

        let ball3 = knock_n_from(f3.standing(), 3);
        let FinalAfterThree::Done(sf) = f3.roll(ball3).unwrap() else {
            panic!("expected Done; duckpin AllDown does NOT earn a fill ball")
        };
        assert_eq!(sf.kind(), FrameKind::AllDown);
        assert_eq!(sf.base_score(), 10);
        assert!(sf.is_final());
        assert_eq!(sf.rolls().len(), 3);
    }
}
