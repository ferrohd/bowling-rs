//! Plays the same sequence of pin counts under ten-pin, candlepin, and duckpin
//! to show how the rulesets diverge.

use bowling::prelude::*;

fn play<R: Ruleset>(rolls: &[u8]) -> u16 {
    let player = Player::new("Demo").unwrap();
    let mut progress = GameBuilder::<R>::new(player).build();

    for &pins in rolls {
        match progress {
            Progress::AwaitingRoll(g) => {
                progress = g.roll_count(pins).unwrap();
            }
            Progress::Complete(_) => break,
        }
    }

    match progress {
        Progress::Complete(g) => {
            let sb = g.scoreboard(0);
            println!("{sb}\n");
            sb.total
        }
        Progress::AwaitingRoll(g) => {
            let sb = g.scoreboard(0);
            println!("{sb}\n");
            println!("  (game still in progress — not enough rolls)");
            sb.total
        }
    }
}

fn main() {
    // A ten-pin game with some strikes and spares.
    // 20 rolls: enough for a full ten-pin game (2 balls/frame).
    // For 3-ball rulesets, the engine just consumes more of the sequence.
    let rolls = [
        // frame1  f2     f3    f4    f5     f6     f7    f8    f9    f10
        10, 7, 3, 9, 0, 10, 0, 8, 8, 2, 0, 6, 10, 10, 10, 6, 4, 10,
        // extra rolls for 3-ball rulesets and final frame fill balls
        3, 4, 2, 5, 3, 1, 7, 0, 4, 6, 2, 8, 3, 5, 7, 2, 4, 1,
    ];

    println!("=== Same rolls, different rules ===\n");
    println!("Feeding the same pin-count sequence into each ruleset.\n");

    println!("--- Ten-Pin ---");
    let tenpin = play::<TenPin>(&rolls);

    println!("--- Candlepin (3 balls/frame, deadwood stays) ---");
    let candlepin = play::<Candlepin>(&rolls);

    println!("--- Duckpin (3 balls/frame, 3-ball clear = no bonus) ---");
    let duckpin = play::<Duckpin>(&rolls);

    println!("Totals: ten-pin={tenpin}, candlepin={candlepin}, duckpin={duckpin}");
}
