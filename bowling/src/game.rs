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
    FinalFillTwo, FinalFrame, FrameNumber, FramePhase, RegAfterOne, RegAfterTwo, RegularFrame,
    ScoredFrame,
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
    current_frame_number: FrameNumber,
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
    fn new_frame(frame_number: FrameNumber) -> Self {
        if frame_number.get() == R::FRAME_COUNT {
            Self::FinalBallOne(FinalFrame::new(frame_number))
        } else {
            Self::RegularBallOne(RegularFrame::new(frame_number))
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
/// The constructor requires the first player, making it structurally
/// impossible to build a game with zero players. Additional players can be
/// added with [`add_player`](GameBuilder::add_player).
///
/// ```
/// use bowling::prelude::*;
///
/// let alice = Player::new("Alice").unwrap();
/// let bob = Player::new("Bob").unwrap();
/// let game = GameBuilder::<TenPin>::new(alice)
///     .add_player(bob)
///     .build();
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
    /// takes the first player up front.
    pub fn new(player: Player) -> Self {
        Self {
            competitors: vec![Competitor {
                player,
                card: ScoreCard::new(),
            }],
            _ruleset: std::marker::PhantomData,
        }
    }

    /// Adds another player to the game.
    pub fn add_player(mut self, player: Player) -> Self {
        self.competitors.push(Competitor {
            player,
            card: ScoreCard::new(),
        });
        self
    }

    /// Builds and starts the game.
    pub fn build(self) -> Game<R, AwaitingRoll> {
        let first_frame = const { FrameNumber::new(1).unwrap() };
        let frame_state = FrameState::new_frame(first_frame);
        Game {
            competitors: self.competitors,
            live: ActiveState {
                current_player: 0,
                current_frame_number: first_frame,
                frame_state,
            },
        }
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
    pub fn current_frame_number(&self) -> FrameNumber {
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
                Ok(advance_turn::<R>(
                    competitors,
                    current_player,
                    current_frame_number,
                ))
            }
        }
    }
}

/// Advances to the next player/frame after a frame completion.
fn advance_turn<R: Ruleset>(
    competitors: Vec<Competitor>,
    current_player: usize,
    current_frame_number: FrameNumber,
) -> Progress<R> {
    let next_player = current_player + 1;

    if next_player < competitors.len() {
        // Same frame, next player
        Progress::AwaitingRoll(Game {
            competitors,
            live: ActiveState {
                current_player: next_player,
                current_frame_number,
                frame_state: FrameState::new_frame(current_frame_number),
            },
        })
    } else {
        // All players bowled this frame
        match current_frame_number.checked_add(1) {
            Some(next_frame) if next_frame.get() <= R::FRAME_COUNT => {
                // Next frame, first player
                Progress::AwaitingRoll(Game {
                    competitors,
                    live: ActiveState {
                        current_player: 0,
                        current_frame_number: next_frame,
                        frame_state: FrameState::new_frame(next_frame),
                    },
                })
            }
            _ => {
                // Game over
                Progress::Complete(Game {
                    competitors,
                    live: (),
                })
            }
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
    standing.into_iter().take(n as usize).collect()
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

    fn frame(n: u8) -> FrameNumber {
        FrameNumber::new(n).unwrap()
    }

    /// Helper: play a full game of all strikes for a single player.
    fn play_perfect_game() -> Game<TenPin, Complete> {
        let alice = Player::new("Alice").unwrap();
        let game = GameBuilder::<TenPin>::new(alice).build();

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
        let bob = Player::new("Bob").unwrap();
        let game = GameBuilder::<TenPin>::new(bob).build();

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
        let alice = Player::new("Alice").unwrap();
        let bob = Player::new("Bob").unwrap();
        let game = GameBuilder::<TenPin>::new(alice).add_player(bob).build();

        assert_eq!(game.current_player().name(), "Alice");
        assert_eq!(game.current_frame_number(), frame(1));

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
        assert_eq!(game.current_frame_number(), frame(1));
    }

    #[test]
    fn empty_player_name_errors() {
        let result = Player::new("");
        assert!(result.is_err());
    }

    #[test]
    fn all_spares_with_five() {
        let charlie = Player::new("Charlie").unwrap();
        let game = GameBuilder::<TenPin>::new(charlie).build();

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
        let alice = Player::new("Alice").unwrap();
        let bob = Player::new("Bob").unwrap();
        let game = GameBuilder::<TenPin>::new(alice).add_player(bob).build();

        assert_eq!(game.competitor_count(), 2);
        assert_eq!(game.player(0).name(), "Alice");
        assert_eq!(game.player(1).name(), "Bob");
        assert_eq!(game.scorecard(0).frames_completed(), 0);
        assert_eq!(game.scorecard(1).frames_completed(), 0);
        assert_eq!(game.competitors().len(), 2);
    }

    #[test]
    fn foul_strike_completes_frame_with_zero_base() {
        // A foul that physically clears all pins: the frame completes
        // (strike transition), but base_score is 0 because Roll::score()
        // returns 0 on fouls.
        let fouler = Player::new("Fouler").unwrap();
        let game = GameBuilder::<TenPin>::new(fouler).build();

        let foul_strike = Roll::foul(PinSet::full::<10>());
        let progress = game.roll(foul_strike).unwrap();

        // Game should advance (frame 1 done, now on frame 2)
        let Progress::AwaitingRoll(game) = progress else {
            panic!("game should continue after one frame")
        };
        assert_eq!(game.current_frame_number(), frame(2));

        // Frame 1 should be recorded with base_score 0
        let card = game.scorecard(0);
        assert_eq!(card.frames_completed(), 1);
        assert_eq!(card.frames()[0].base_score(), 0);
    }

    #[test]
    fn candlepin_full_game_with_three_ball_frames() {
        // Play a complete Candlepin game where frames go to 3 balls.
        // Frame pattern: each frame knocks 3, then 3, then 3 (open, 9 pins).
        // Final frame: same pattern (3, 3, 3 = open, 9).
        // No strikes or spares → no bonuses. Total = 9 × 10 = 90.
        let candle = Player::new("Candle").unwrap();
        let game = GameBuilder::<Candlepin>::new(candle).build();

        let mut progress = Progress::AwaitingRoll(game);
        // 10 frames × 3 balls = 30 rolls
        for _ in 0..30 {
            match progress {
                Progress::AwaitingRoll(g) => {
                    progress = g.roll_count(3).unwrap();
                }
                Progress::Complete(_) => break,
            }
        }

        let Progress::Complete(game) = progress else {
            panic!("candlepin game should complete after 30 rolls of 3")
        };

        let sb = game.scoreboard(0);
        assert_eq!(sb.total, 90); // 10 frames × 9 pins each
    }

    #[test]
    fn duckpin_alldown_gets_no_bonus() {
        // Duckpin: frame 1 clears all pins in 3 balls (AllDown).
        // Frame 2 is a simple open. AllDown earns NO bonus in duckpin,
        // so frame 1 score should be exactly 10 (flat), not 10 + frame 2 rolls.
        let duck = Player::new("Duck").unwrap();
        let game = GameBuilder::<Duckpin>::new(duck).build();

        // Frame 1: 3 + 4 + 3 = 10 (all down in 3 balls)
        let progress = game.roll_count(3).unwrap();
        let Progress::AwaitingRoll(g) = progress else {
            panic!("should continue")
        };
        let progress = g.roll_count(4).unwrap();
        let Progress::AwaitingRoll(g) = progress else {
            panic!("should continue")
        };
        let progress = g.roll_count(3).unwrap();
        let Progress::AwaitingRoll(g) = progress else {
            panic!("should continue; frame 1 done, frame 2 starts")
        };
        assert_eq!(g.current_frame_number(), frame(2));

        // Frame 2: 5 + 2 + 1 = 8 (open, 3 balls in duckpin)
        let progress = g.roll_count(5).unwrap();
        let Progress::AwaitingRoll(g) = progress else {
            panic!("should continue")
        };
        let progress = g.roll_count(2).unwrap();
        let Progress::AwaitingRoll(g) = progress else {
            panic!("should continue; duckpin has 3 balls per frame")
        };
        let progress = g.roll_count(1).unwrap();

        // Check scoreboard: frame 1 should be 10 flat (no bonus)
        let g = match progress {
            Progress::AwaitingRoll(g) => g,
            Progress::Complete(_) => panic!("game should not be complete after 2 frames"),
        };
        assert_eq!(g.current_frame_number(), frame(3));
        let sb = g.scoreboard(0);

        // Frame 1: AllDown, base 10, bonus 0 (not spare)
        let FrameScore::Resolved {
            base,
            bonus,
            cumulative,
            ..
        } = &sb.frames[0]
        else {
            panic!("frame 1 should be resolved")
        };
        assert_eq!(*base, 10);
        assert_eq!(*bonus, 0); // AllDown → no bonus
        assert_eq!(*cumulative, 10);

        // Frame 2: Open, base 8
        let FrameScore::Resolved {
            base, cumulative, ..
        } = &sb.frames[1]
        else {
            panic!("frame 2 should be resolved")
        };
        assert_eq!(*base, 8);
        assert_eq!(*cumulative, 18); // 10 + 8
    }
}
