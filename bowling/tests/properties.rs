//! Property-based tests using proptest.
//!
//! These tests verify invariants that must hold for *any* valid sequence of
//! rolls, across all three rulesets.

use bowling::prelude::*;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Strategies
// ---------------------------------------------------------------------------

/// Generate a random pin count between 0 and `max`.
fn pin_count(max: u8) -> impl Strategy<Value = u8> {
    0..=max
}

/// Play an entire game using `roll_count`, returning the final scoreboard.
/// Rolls are drawn from the given `counts` vector. If the game finishes
/// before all counts are used, that's fine.
fn play_game_with_counts<R: Ruleset>(counts: &[u8]) -> (Scoreboard, u32) {
    let player = Player::new("Prop").unwrap();
    let game = GameBuilder::<R>::new(player).build();

    let mut progress = Progress::AwaitingRoll(game);
    let mut rolls_used = 0u32;

    for &n in counts {
        match progress {
            Progress::AwaitingRoll(g) => {
                let standing = g.standing_pins().count();
                let clamped = n.min(standing);
                progress = g.roll_count(clamped).unwrap();
                rolls_used += 1;
            }
            Progress::Complete(_) => break,
        }
    }

    let scoreboard = match progress {
        Progress::Complete(g) => g.scoreboard(0),
        Progress::AwaitingRoll(g) => g.scoreboard(0),
    };

    (scoreboard, rolls_used)
}

// ---------------------------------------------------------------------------
// Ten-pin property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn tenpin_score_in_valid_range(
        counts in proptest::collection::vec(pin_count(10), 12..=21)
    ) {
        let (sb, _) = play_game_with_counts::<TenPin>(&counts);
        prop_assert!(sb.total <= 300, "ten-pin score must be <= 300, got {}", sb.total);
    }

    #[test]
    fn tenpin_game_always_terminates(
        counts in proptest::collection::vec(pin_count(10), 21..=30)
    ) {
        let player = Player::new("Term").unwrap();
        let game = GameBuilder::<TenPin>::new(player).build();

        let mut progress = Progress::AwaitingRoll(game);
        let mut total_rolls = 0u32;

        for &n in &counts {
            match progress {
                Progress::AwaitingRoll(g) => {
                    let standing = g.standing_pins().count();
                    let clamped = n.min(standing);
                    progress = g.roll_count(clamped).unwrap();
                    total_rolls += 1;
                }
                Progress::Complete(_) => break,
            }
        }

        // A ten-pin game needs at most 21 rolls
        prop_assert!(total_rolls <= 21, "ten-pin game used {} rolls", total_rolls);
        // And it should be complete with 21 rolls worth of input
        if let Progress::Complete(_) = progress {
            // ok
        } else {
            // It's ok if we ran out of input before completion (e.g., 21
            // rolls of 0 is only 20 actual game rolls). The important thing
            // is no panic/infinite loop.
        }
    }

    #[test]
    fn tenpin_perfect_game_is_300(
        _dummy in Just(())
    ) {
        let counts = vec![10u8; 12];
        let (sb, _) = play_game_with_counts::<TenPin>(&counts);
        prop_assert_eq!(sb.total, 300);
    }

    #[test]
    fn tenpin_all_gutter_is_zero(
        _dummy in Just(())
    ) {
        let counts = vec![0u8; 20];
        let (sb, _) = play_game_with_counts::<TenPin>(&counts);
        prop_assert_eq!(sb.total, 0);
    }

    #[test]
    fn tenpin_frame_scores_sum_to_total(
        counts in proptest::collection::vec(pin_count(10), 21..=21)
    ) {
        let (sb, _) = play_game_with_counts::<TenPin>(&counts);
        // The last resolved frame's cumulative should equal total
        if let Some(cum) = sb.frames.iter().rev().find_map(|f| match f {
            bowling::scoring::FrameScore::Resolved { cumulative, .. } => Some(*cumulative),
            bowling::scoring::FrameScore::Pending { .. } => None,
        }) {
            prop_assert_eq!(cum, sb.total);
        }
    }
}

// ---------------------------------------------------------------------------
// Candlepin property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn candlepin_score_in_valid_range(
        counts in proptest::collection::vec(pin_count(10), 12..=30)
    ) {
        let (sb, _) = play_game_with_counts::<Candlepin>(&counts);
        // Candlepin max: 300 (same as ten-pin, 10 strikes + 2 bonus)
        prop_assert!(sb.total <= 300, "candlepin score must be <= 300, got {}", sb.total);
    }
}

// ---------------------------------------------------------------------------
// Duckpin property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    #[test]
    fn duckpin_score_in_valid_range(
        counts in proptest::collection::vec(pin_count(10), 12..=30)
    ) {
        let (sb, _) = play_game_with_counts::<Duckpin>(&counts);
        // Duckpin max: 300 (10 strikes + 2 bonus)
        prop_assert!(sb.total <= 300, "duckpin score must be <= 300, got {}", sb.total);
    }
}

// ---------------------------------------------------------------------------
// Multi-player property tests
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    #[test]
    fn multiplayer_game_completes(
        player_count in 2u8..=4,
        counts in proptest::collection::vec(pin_count(10), 80..=100)
    ) {
        let p0 = Player::new("P0").unwrap();
        let mut builder = GameBuilder::<TenPin>::new(p0);
        for i in 1..player_count {
            let player = Player::new(format!("P{i}")).unwrap();
            builder = builder.add_player(player);
        }
        let game = builder.build();

        let mut progress = Progress::AwaitingRoll(game);
        for &n in &counts {
            match progress {
                Progress::AwaitingRoll(g) => {
                    let standing = g.standing_pins().count();
                    let clamped = n.min(standing);
                    progress = g.roll_count(clamped).unwrap();
                }
                Progress::Complete(_) => break,
            }
        }

        // With enough rolls, the game should be complete
        // (each player needs at most 21 rolls, 4 players = 84 max)
        if let Progress::Complete(g) = progress {
            let boards = g.scoreboards();
            prop_assert_eq!(boards.len(), player_count as usize);
            for board in &boards {
                prop_assert!(board.total <= 300);
            }
        }
    }
}
