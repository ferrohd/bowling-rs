//! Game-level typestate machine with multiplayer turn rotation.
//!
//! [`Game`] is parameterised by a [`Ruleset`] `R` and a phase `P`:
//!
//! - [`AwaitingRoll`]: the game is in progress and expecting a delivery.
//!   The phase payload ([`ActiveState`]) carries the turn cursor and the
//!   live frame state (non-optional, guaranteed by the type).
//! - [`Complete`]: the game has ended, only read-only inspection is
//!   available. The phase payload is `()`.
//!
//! Transitions go through the [`Progress`] enum so that callers can pattern-
//! match on the outcome of each roll. Calling `.roll()` on a `Complete` game
//! is a **compile error**.

use crate::error::BowlingError;
use crate::frame::{
    BallOne, BallThree, BallTwo, FinalAfterOne, FinalAfterThree, FinalAfterTwo, FinalFillThree,
    FinalFillTwo, FinalFrame, FramePhase, RegAfterOne, RegAfterTwo, RegularFrame, ScoredFrame,
};
use crate::pins::PinSet;
use crate::player::Player;
use crate::roll::Roll;
use crate::ruleset::Ruleset;
use crate::scorecard::ScoreCard;
use crate::scoring::{self, Scoreboard};

// =========================================================================
// GamePhase trait: per-phase payload via associated type
// =========================================================================

/// Determines the per-phase payload stored inside a [`Game`].
///
/// - [`AwaitingRoll`] carries an [`ActiveState`] (turn cursor + live frame).
/// - [`Complete`] carries `()`.
///
/// This eliminates the `Option<FrameState>` that previously required runtime
/// `expect` assertions.
pub trait GamePhase<R: Ruleset>: std::fmt::Debug {
    /// The per-phase data stored in the game.
    type Live: std::fmt::Debug;
}

// =========================================================================
// Phase markers
// =========================================================================

/// The game is in progress and awaiting the next delivery.
#[derive(Debug)]
pub struct AwaitingRoll;

/// All players have completed all frames.
#[derive(Debug)]
pub struct Complete;

impl<R: Ruleset> GamePhase<R> for AwaitingRoll {
    type Live = ActiveState<R>;
}

impl<R: Ruleset> GamePhase<R> for Complete {
    type Live = ();
}

// =========================================================================
// ActiveState: the non-optional live-game payload
// =========================================================================

/// Live game state for an in-progress game.
///
/// Contains the turn cursor and the in-flight frame state. Stored directly
/// (not behind `Option`) inside `Game<R, AwaitingRoll>`, so every access is
/// infallible.
///
/// Fields are private. This type is public only because it appears as an
/// associated type on [`GamePhase`]. External code never constructs or
/// destructures it.
#[derive(Debug)]
pub struct ActiveState<R: Ruleset> {
    /// Index of the player currently bowling.
    current_player: usize,
    /// The current frame number being bowled (1-indexed).
    current_frame_number: u8,
    /// The in-flight frame typestate (never absent while awaiting a roll).
    frame_state: FrameState<R>,
}

// =========================================================================
// Competitor: binds identity to per-game state
// =========================================================================

/// A participant in a game: their identity ([`Player`]) and their per-game
/// score record ([`ScoreCard`]).
#[derive(Debug, Clone)]
pub struct Competitor {
    player: Player,
    card: ScoreCard,
}

impl Competitor {
    /// Returns the player's identity.
    pub fn player(&self) -> &Player {
        &self.player
    }

    /// Returns the player's score card for this game.
    pub fn card(&self) -> &ScoreCard {
        &self.card
    }
}

// =========================================================================
// Internal frame state (runtime envelope around typestate frames)
// =========================================================================

/// Runtime wrapper that erases the frame-phase type parameter so we can
/// store the "current frame" inside `Game` without infecting `Game`'s own
/// type signature. This is the "runtime driver over typestate core" bridge.
#[derive(Debug)]
enum FrameState<R: Ruleset> {
    // Regular frame phases
    RegularBallOne(RegularFrame<R, BallOne>),
    RegularBallTwo(RegularFrame<R, BallTwo>),
    RegularBallThree(RegularFrame<R, BallThree>),
    // Final frame phases
    FinalBallOne(FinalFrame<R, BallOne>),
    FinalBallTwo(FinalFrame<R, BallTwo>),
    FinalBallThree(FinalFrame<R, BallThree>),
    FinalFillTwo(FinalFrame<R, FinalFillTwo>),
    FinalFillThree(FinalFrame<R, FinalFillThree>),
}

impl<R: Ruleset> FrameState<R> {
    /// Returns the pins currently standing in the active frame.
    fn standing(&self) -> PinSet {
        match self {
            Self::RegularBallOne(f) => f.standing(),
            Self::RegularBallTwo(f) => f.standing(),
            Self::RegularBallThree(f) => f.standing(),
            Self::FinalBallOne(f) => f.standing(),
            Self::FinalBallTwo(f) => f.standing(),
            Self::FinalBallThree(f) => f.standing(),
            Self::FinalFillTwo(f) => f.standing(),
            Self::FinalFillThree(f) => f.standing(),
        }
    }

    /// Returns the rolls recorded so far in the active frame.
    fn rolls(&self) -> &[Roll] {
        match self {
            Self::RegularBallOne(f) => f.phase().rolls(),
            Self::RegularBallTwo(f) => f.phase().rolls(),
            Self::RegularBallThree(f) => f.phase().rolls(),
            Self::FinalBallOne(f) => f.phase().rolls(),
            Self::FinalBallTwo(f) => f.phase().rolls(),
            Self::FinalBallThree(f) => f.phase().rolls(),
            Self::FinalFillTwo(f) => f.phase().rolls(),
            Self::FinalFillThree(f) => f.phase().rolls(),
        }
    }

    /// Returns which ball number (1-indexed) is about to be thrown.
    fn ball_number(&self) -> u8 {
        match self {
            Self::RegularBallOne(_) | Self::FinalBallOne(_) => 1,
            Self::RegularBallTwo(_) | Self::FinalBallTwo(_) | Self::FinalFillTwo(_) => 2,
            Self::RegularBallThree(_) | Self::FinalBallThree(_) | Self::FinalFillThree(_) => 3,
        }
    }

    /// Create the appropriate frame state for the given frame number.
    fn new_frame(frame_number: u8) -> Result<Self, BowlingError> {
        if frame_number == R::FRAME_COUNT {
            Ok(Self::FinalBallOne(FinalFrame::new(frame_number)?))
        } else {
            Ok(Self::RegularBallOne(RegularFrame::new(frame_number)?))
        }
    }

    /// Deliver a ball, consuming the current state and producing either a
    /// new in-progress state or a completed [`ScoredFrame`].
    ///
    /// Every match arm is exhaustive (no `unreachable!` needed) because
    /// each frame phase returns a precise outcome type covering only its
    /// reachable transitions.
    fn roll(self, delivery: Roll) -> Result<FrameRollResult<R>, BowlingError> {
        match self {
            // --- Regular ---
            Self::RegularBallOne(f) => match f.roll(delivery)? {
                RegAfterOne::Continue(f2) => {
                    Ok(FrameRollResult::Continue(Self::RegularBallTwo(f2)))
                }
                RegAfterOne::Done(sf) => Ok(FrameRollResult::Done(sf)),
            },
            Self::RegularBallTwo(f) => match f.roll(delivery)? {
                RegAfterTwo::Continue(f3) => {
                    Ok(FrameRollResult::Continue(Self::RegularBallThree(f3)))
                }
                RegAfterTwo::Done(sf) => Ok(FrameRollResult::Done(sf)),
            },
            Self::RegularBallThree(f) => Ok(FrameRollResult::Done(f.roll(delivery)?)),
            // --- Final ---
            Self::FinalBallOne(f) => match f.roll(delivery)? {
                FinalAfterOne::ToBallTwo(f2) => {
                    Ok(FrameRollResult::Continue(Self::FinalBallTwo(f2)))
                }
                FinalAfterOne::ToFillTwo(f2) => {
                    Ok(FrameRollResult::Continue(Self::FinalFillTwo(f2)))
                }
            },
            Self::FinalBallTwo(f) => match f.roll(delivery)? {
                FinalAfterTwo::ToBallThree(f3) => {
                    Ok(FrameRollResult::Continue(Self::FinalBallThree(f3)))
                }
                FinalAfterTwo::ToFillThree(f3) => {
                    Ok(FrameRollResult::Continue(Self::FinalFillThree(f3)))
                }
                FinalAfterTwo::Done(sf) => Ok(FrameRollResult::Done(sf)),
            },
            Self::FinalBallThree(f) => match f.roll(delivery)? {
                FinalAfterThree::ToFillThree(f3) => {
                    Ok(FrameRollResult::Continue(Self::FinalFillThree(f3)))
                }
                FinalAfterThree::Done(sf) => Ok(FrameRollResult::Done(sf)),
            },
            Self::FinalFillTwo(f) => {
                let f3 = f.roll(delivery)?;
                Ok(FrameRollResult::Continue(Self::FinalFillThree(f3)))
            }
            Self::FinalFillThree(f) => Ok(FrameRollResult::Done(f.roll(delivery)?)),
        }
    }
}

/// Result of delivering a ball to the current frame.
enum FrameRollResult<R: Ruleset> {
    /// The frame continues with more deliveries.
    Continue(FrameState<R>),
    /// The frame is complete.
    Done(ScoredFrame),
}

// =========================================================================
// Game
// =========================================================================

/// A bowling game for one or more players.
///
/// Parameterised by:
/// - `R`: the [`Ruleset`] (e.g. [`TenPin`], [`Candlepin`], [`Duckpin`]).
/// - `P`: the phase, either [`AwaitingRoll`] or [`Complete`].
///
/// The game manages turn rotation automatically: after each player finishes
/// a frame, the next player bowls the same frame, and so on.
///
/// Player identity ([`Player`]) is separated from per-game state
/// ([`ScoreCard`]). The [`Game`] owns both via [`Competitor`].
///
/// [`TenPin`]: crate::ruleset::TenPin
/// [`Candlepin`]: crate::ruleset::Candlepin
/// [`Duckpin`]: crate::ruleset::Duckpin
pub struct Game<R: Ruleset, P: GamePhase<R>> {
    /// All competitors (player identity + per-game score card).
    competitors: Vec<Competitor>,
    /// Phase-specific live state.
    live: P::Live,
}

impl<R: Ruleset, P: GamePhase<R>> std::fmt::Debug for Game<R, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Game")
            .field("ruleset", &R::name())
            .field("competitors", &self.competitors.len())
            .finish_non_exhaustive()
    }
}

// =========================================================================
// Progress enum
// =========================================================================

/// The outcome of a [`Game::roll`] call.
#[derive(Debug)]
pub enum Progress<R: Ruleset> {
    /// The game continues, more rolls needed.
    AwaitingRoll(Game<R, AwaitingRoll>),
    /// All players have finished all frames.
    Complete(Game<R, Complete>),
}

// =========================================================================
// Builder
// =========================================================================

/// Builder for constructing a [`Game`].
///
/// The constructor requires the first player's name, making it structurally
/// impossible to build a game with zero players. Additional players can be
/// added with [`add_player`](GameBuilder::add_player).
///
/// ```
/// use bowling_rs::prelude::*;
///
/// let game = GameBuilder::<TenPin>::new("Alice")
///     .unwrap()
///     .add_player("Bob")
///     .unwrap()
///     .build()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct GameBuilder<R: Ruleset> {
    competitors: Vec<Competitor>,
    _ruleset: std::marker::PhantomData<R>,
}

impl<R: Ruleset> GameBuilder<R> {
    /// Creates a new game builder with the first player.
    ///
    /// At least one player is required to create a game, so the constructor
    /// takes the first player's name up front.
    ///
    /// # Errors
    ///
    /// Returns [`BowlingError::EmptyPlayerName`] if `name` is empty.
    pub fn new(name: impl Into<String>) -> Result<Self, BowlingError> {
        Ok(Self {
            competitors: vec![Competitor {
                player: Player::new(name)?,
                card: ScoreCard::new(),
            }],
            _ruleset: std::marker::PhantomData,
        })
    }

    /// Adds another player to the game.
    ///
    /// # Errors
    ///
    /// Returns [`BowlingError::EmptyPlayerName`] if `name` is empty.
    pub fn add_player(mut self, name: impl Into<String>) -> Result<Self, BowlingError> {
        self.competitors.push(Competitor {
            player: Player::new(name)?,
            card: ScoreCard::new(),
        });
        Ok(self)
    }

    /// Builds and starts the game.
    ///
    /// # Errors
    ///
    /// Returns [`BowlingError::TooManyPins`] if the ruleset declares a pin
    /// count exceeding 16.
    pub fn build(self) -> Result<Game<R, AwaitingRoll>, BowlingError> {
        let frame_state = FrameState::new_frame(1)?;
        Ok(Game {
            competitors: self.competitors,
            live: ActiveState {
                current_player: 0,
                current_frame_number: 1,
                frame_state,
            },
        })
    }
}

// =========================================================================
// Shared accessors (both phases)
// =========================================================================

impl<R: Ruleset, P: GamePhase<R>> Game<R, P> {
    /// Returns all competitors (player identity + score card).
    pub fn competitors(&self) -> &[Competitor] {
        &self.competitors
    }

    /// Returns the number of competitors.
    pub fn competitor_count(&self) -> usize {
        self.competitors.len()
    }

    /// Returns the [`Player`] at the given index.
    pub fn player(&self, index: usize) -> &Player {
        &self.competitors[index].player
    }

    /// Returns the [`ScoreCard`] at the given index.
    pub fn scorecard(&self, index: usize) -> &ScoreCard {
        &self.competitors[index].card
    }

    /// Computes the [`Scoreboard`] for the player at the given index.
    pub fn scoreboard(&self, index: usize) -> Scoreboard {
        scoring::compute_scoreboard::<R>(self.competitors[index].card.frames())
    }

    /// Computes scoreboards for all players.
    pub fn scoreboards(&self) -> Vec<Scoreboard> {
        self.competitors
            .iter()
            .map(|c| scoring::compute_scoreboard::<R>(c.card.frames()))
            .collect()
    }
}

// =========================================================================
// Game<R, AwaitingRoll>: the active game
// =========================================================================

impl<R: Ruleset> Game<R, AwaitingRoll> {
    /// Returns the current player's identity.
    pub fn current_player(&self) -> &Player {
        &self.competitors[self.live.current_player].player
    }

    /// Returns the current frame number (1-indexed).
    pub fn current_frame_number(&self) -> u8 {
        self.live.current_frame_number
    }

    /// Returns the current player index (0-indexed).
    pub fn current_player_index(&self) -> usize {
        self.live.current_player
    }

    /// Returns the rolls recorded so far in the current (in-progress) frame.
    ///
    /// This is empty at the start of a frame, contains 1 roll after the first
    /// ball, etc. Useful for rendering partial scoreboards.
    pub fn current_frame_rolls(&self) -> &[Roll] {
        self.live.frame_state.rolls()
    }

    /// Returns which ball number (1-indexed) is about to be thrown in the
    /// current frame.
    pub fn current_ball_number(&self) -> u8 {
        self.live.frame_state.ball_number()
    }

    /// Convenience: delivers a clean (non-foul) ball knocking `n` pins,
    /// using a simple lowest-bit-first bitmask over the standing pins.
    ///
    /// This is a helper for the common case where you just want to say
    /// "knock down N pins" without specifying which individual pins.
    pub fn roll_count(self, n: u8) -> Result<Progress<R>, BowlingError> {
        let standing = self.standing_pins();
        let knocked = pick_n_from(standing, n);
        let delivery = Roll::clean(knocked);
        self.roll(delivery)
    }

    /// Returns the pins currently standing for the active frame.
    pub fn standing_pins(&self) -> PinSet {
        self.live.frame_state.standing()
    }

    /// Delivers a ball.
    ///
    /// Consumes `self` and returns a [`Progress`]: either the game
    /// continues (`AwaitingRoll`) or is finished (`Complete`).
    pub fn roll(self, delivery: Roll) -> Result<Progress<R>, BowlingError> {
        let mut competitors = self.competitors;
        let ActiveState {
            current_player,
            current_frame_number,
            frame_state,
        } = self.live;

        match frame_state.roll(delivery)? {
            FrameRollResult::Continue(next_state) => {
                // Same player, same frame, more balls
                Ok(Progress::AwaitingRoll(Game {
                    competitors,
                    live: ActiveState {
                        current_player,
                        current_frame_number,
                        frame_state: next_state,
                    },
                }))
            }
            FrameRollResult::Done(scored) => {
                // Frame complete for current player
                competitors[current_player].card.push_frame(scored);
                advance_turn::<R>(competitors, current_player, current_frame_number)
            }
        }
    }
}

/// Advances to the next player/frame after a frame completion.
fn advance_turn<R: Ruleset>(
    competitors: Vec<Competitor>,
    current_player: usize,
    current_frame_number: u8,
) -> Result<Progress<R>, BowlingError> {
    let next_player = current_player + 1;

    if next_player < competitors.len() {
        // Same frame, next player
        Ok(Progress::AwaitingRoll(Game {
            competitors,
            live: ActiveState {
                current_player: next_player,
                current_frame_number,
                frame_state: FrameState::new_frame(current_frame_number)?,
            },
        }))
    } else {
        // All players bowled this frame
        let next_frame = current_frame_number + 1;
        if next_frame > R::FRAME_COUNT {
            // Game over
            Ok(Progress::Complete(Game {
                competitors,
                live: (),
            }))
        } else {
            // Next frame, first player
            Ok(Progress::AwaitingRoll(Game {
                competitors,
                live: ActiveState {
                    current_player: 0,
                    current_frame_number: next_frame,
                    frame_state: FrameState::new_frame(next_frame)?,
                },
            }))
        }
    }
}

// =========================================================================
// Game<R, Complete>: read-only final state
// =========================================================================

impl<R: Ruleset> Game<R, Complete> {
    /// Returns the winner(s). In case of a tie, all tying players are
    /// returned.
    pub fn winners(&self) -> Vec<&Player> {
        let boards = self.scoreboards();
        let max = boards.iter().map(|b| b.total).max().unwrap_or(0);
        self.competitors
            .iter()
            .zip(boards.iter())
            .filter(|(_, b)| b.total == max)
            .map(|(c, _)| &c.player)
            .collect()
    }
}

// =========================================================================
// Helpers
// =========================================================================

/// Pick `n` pins from the standing set using a simple lowest-bit-first
/// strategy.
fn pick_n_from(standing: PinSet, n: u8) -> PinSet {
    let mut result = PinSet::EMPTY;
    for (count, idx) in standing.into_iter().enumerate() {
        if count >= n as usize {
            break;
        }
        result = result.insert(idx);
    }
    result
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pins::PinSet;
    use crate::roll::Roll;
    use crate::ruleset::{Candlepin, Duckpin, TenPin};
    use crate::scoring::FrameScore;

    /// Helper: play a full game of all strikes for a single player.
    fn play_perfect_game() -> Game<TenPin, Complete> {
        let game = GameBuilder::<TenPin>::new("Alice")
            .unwrap()
            .build()
            .unwrap();

        let mut progress = Progress::AwaitingRoll(game);
        for _ in 0..12 {
            match progress {
                Progress::AwaitingRoll(g) => {
                    progress = g.roll_count(10).unwrap();
                }
                Progress::Complete(_) => break,
            }
        }
        match progress {
            Progress::Complete(g) => g,
            Progress::AwaitingRoll(_) => panic!("game should be complete after 12 strikes"),
        }
    }

    #[test]
    fn perfect_game_scores_300() {
        let game = play_perfect_game();
        let sb = game.scoreboard(0);
        assert_eq!(sb.total, 300);
    }

    #[test]
    fn all_gutter_scores_zero() {
        let game = GameBuilder::<TenPin>::new("Bob")
            .unwrap()
            .build()
            .unwrap();

        let mut progress = Progress::AwaitingRoll(game);
        for _ in 0..20 {
            match progress {
                Progress::AwaitingRoll(g) => {
                    progress = g.roll_count(0).unwrap();
                }
                Progress::Complete(_) => break,
            }
        }
        match progress {
            Progress::Complete(g) => {
                assert_eq!(g.scoreboard(0).total, 0);
            }
            Progress::AwaitingRoll(_) => panic!("game should be complete"),
        }
    }

    #[test]
    fn two_player_game_alternates() {
        let game = GameBuilder::<TenPin>::new("Alice")
            .unwrap()
            .add_player("Bob")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(game.current_player().name(), "Alice");
        assert_eq!(game.current_frame_number(), 1);

        let progress = game.roll_count(3).unwrap();
        let Progress::AwaitingRoll(game) = progress else {
            panic!("should continue")
        };
        assert_eq!(game.current_player().name(), "Alice");

        let progress = game.roll_count(4).unwrap();
        let Progress::AwaitingRoll(game) = progress else {
            panic!("should continue")
        };
        assert_eq!(game.current_player().name(), "Bob");
        assert_eq!(game.current_frame_number(), 1);
    }

    #[test]
    fn empty_player_name_errors() {
        let result = GameBuilder::<TenPin>::new("");
        assert!(result.is_err());
    }

    #[test]
    fn all_spares_with_five() {
        let game = GameBuilder::<TenPin>::new("Charlie")
            .unwrap()
            .build()
            .unwrap();

        let mut progress = Progress::AwaitingRoll(game);
        for i in 0..21 {
            match progress {
                Progress::AwaitingRoll(g) => {
                    progress = g.roll_count(5).unwrap();
                }
                Progress::Complete(_) => {
                    assert_eq!(i, 21, "should complete after 21 rolls");
                    break;
                }
            }
        }
        match progress {
            Progress::Complete(g) => {
                assert_eq!(g.scoreboard(0).total, 150);
            }
            Progress::AwaitingRoll(_) => panic!("game should be complete"),
        }
    }

    #[test]
    fn competitor_accessors_work() {
        let game = GameBuilder::<TenPin>::new("Alice")
            .unwrap()
            .add_player("Bob")
            .unwrap()
            .build()
            .unwrap();

        assert_eq!(game.competitor_count(), 2);
        assert_eq!(game.player(0).name(), "Alice");
        assert_eq!(game.player(1).name(), "Bob");
        assert_eq!(game.scorecard(0).frames_completed(), 0);
        assert_eq!(game.scorecard(1).frames_completed(), 0);
        assert_eq!(game.competitors().len(), 2);
    }
}
