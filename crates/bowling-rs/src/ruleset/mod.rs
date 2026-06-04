//! Bowling ruleset abstraction and concrete implementations.
//!
//! A [`Ruleset`] defines the parameters that distinguish one bowling variant
//! from another: pin count, frame count, balls per frame, deadwood policy,
//! bonus scoring, and optional pin geometry for split detection.

mod candlepin;
mod duckpin;
mod tenpin;

pub use candlepin::Candlepin;
pub use duckpin::Duckpin;
pub use tenpin::TenPin;

use crate::geometry::PinGeometry;
use crate::pins::PinSet;

// ---------------------------------------------------------------------------
// DeadwoodPolicy
// ---------------------------------------------------------------------------

/// How fallen pins (deadwood) are handled between deliveries within a frame.
///
/// This is declarative metadata. The game engine's abstract state machine
/// (which pins are standing, which were knocked, scoring) is identical for
/// both policies. The difference is purely physical:
///
/// - `Cleared`: fallen pins are swept off the deck between deliveries. The
///   pin deck shows only still-standing pins.
/// - `Remains`: fallen pins stay on the deck and can physically interfere
///   with subsequent deliveries.
///
/// UI and rendering layers can query this via [`Ruleset::deadwood()`] to
/// decide how to visualise the pin deck. The scoring engine does not branch
/// on this value.
///
/// **Foul semantics note:** in `Cleared` variants (ten-pin), fouled pins are
/// typically re-spotted by the machinery. In `Remains` variants (candlepin),
/// fouled pins stay down. The current engine delegates foul handling to the
/// caller: the [`Roll`](crate::roll::Roll) reports what was physically
/// knocked, and scoring zeroes the delivery separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadwoodPolicy {
    /// Deadwood is cleared before the next delivery (ten-pin, duckpin).
    /// The pin deck shows only still-standing pins.
    Cleared,
    /// Deadwood remains on the deck (candlepin). Fallen pins stay where they
    /// are and can interfere with subsequent deliveries.
    Remains,
}

// ---------------------------------------------------------------------------
// BonusScheme
// ---------------------------------------------------------------------------

/// How bonus scoring works for strikes and spares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusScheme {
    /// Traditional bonus: a strike adds the next N deliveries' pin counts to
    /// the frame score; a spare adds the next M deliveries' pin counts.
    Traditional {
        /// Number of bonus balls after a strike (typically 2).
        strike_bonus_balls: u8,
        /// Number of bonus balls after a spare (typically 1).
        spare_bonus_balls: u8,
    },
}

// ---------------------------------------------------------------------------
// Ruleset trait
// ---------------------------------------------------------------------------

/// Defines the rules for a bowling variant.
///
/// Implemented as a unit struct with associated constants for zero-cost
/// compile-time configuration. The trait is designed for **static dispatch**:
/// `Game<R: Ruleset, Phase>` is monomorphized per variant.
pub trait Ruleset: Sized + Clone + std::fmt::Debug + 'static {
    /// Number of pins in a full rack.
    const PIN_COUNT: u8;

    /// Number of frames in a game.
    const FRAME_COUNT: u8;

    /// Maximum number of deliveries in a regular (non-final) frame.
    const BALLS_PER_FRAME: u8;

    /// How deadwood is handled within a frame.
    const DEADWOOD: DeadwoodPolicy;

    /// Bonus scoring scheme.
    const BONUS: BonusScheme;

    /// Whether knocking all pins down using *all* allowed balls (without doing
    /// so on the first ball) counts as a spare with bonus, or is scored flat.
    ///
    /// - `true` (ten-pin, candlepin): clearing the deck in ≤ `BALLS_PER_FRAME`
    ///   balls (but not the first) is a spare.
    /// - `false` (duckpin): clearing with all 3 balls is just a flat 10, no
    ///   bonus ("ten").
    const ALL_DOWN_IS_SPARE: bool;

    /// Optional pin geometry for split detection. Return `None` to opt out.
    fn geometry() -> Option<&'static PinGeometry>;

    /// Human-readable name of the variant.
    fn name() -> &'static str;

    /// Returns a full rack of pins for this ruleset.
    ///
    /// Implementors should call [`PinSet::full`] with the appropriate
    /// const-generic pin count, e.g. `PinSet::full::<10>()`. This makes
    /// the pin count a compile-time constant, eliminating runtime
    /// validation from frame construction.
    fn full_rack() -> PinSet;

    /// Returns the deadwood policy for this ruleset.
    ///
    /// Convenience accessor for [`Self::DEADWOOD`]. UI and rendering layers
    /// can use this to decide how to visualise the pin deck between
    /// deliveries.
    fn deadwood() -> DeadwoodPolicy {
        Self::DEADWOOD
    }
}
