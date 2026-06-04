//! A two-player ten-pin game with a mix of strikes, spares, and open frames.

use bowling_rs::prelude::*;

fn main() {
    println!("=== Two-Player Game ===\n");

    let game = GameBuilder::<TenPin>::new("Alice")
        .unwrap()
        .add_player("Bob")
        .unwrap()
        .build()
        .unwrap();

    // Pre-scripted rolls for both players interleaved by the engine's
    // turn rotation. Alice bowls her frame, then Bob bowls his, repeat.
    //
    // Alice: X, 7/, 9-0, X, 0/8, 8/, X, X, 9-0, X-X-X  = ?
    // Bob:   8-1, X, 7/, 5-3, X, X, 9-0, 7/, 8-1, 9/-5  = ?
    //
    // But we don't need to think about interleaving — the engine handles
    // turn rotation. We just feed rolls and check current_player().

    let rolls: &[(u8, &str)] = &[
        // Frame 1
        (10, "Strike!"), // Alice — strike
        (8, ""),         // Bob — 8
        (1, ""),         // Bob — 1 (open, 9)
        // Frame 2
        (7, ""),         // Alice — 7
        (3, "Spare!"),   // Alice — spare
        (10, "Strike!"), // Bob — strike
        // Frame 3
        (9, ""),       // Alice — 9
        (0, "Miss"),   // Alice — gutter (open, 9)
        (7, ""),       // Bob — 7
        (3, "Spare!"), // Bob — spare
        // Frame 4
        (10, "Strike!"), // Alice — strike
        (5, ""),         // Bob — 5
        (3, ""),         // Bob — 3 (open, 8)
        // Frame 5
        (0, "Gutter"),   // Alice — gutter
        (8, ""),         // Alice — 8 (open, 8)
        (10, "Strike!"), // Bob — strike
        // Frame 6
        (8, ""),         // Alice — 8
        (2, "Spare!"),   // Alice — spare
        (10, "Strike!"), // Bob — strike
        // Frame 7
        (10, "Strike!"), // Alice — strike
        (9, ""),         // Bob — 9
        (0, "Miss"),     // Bob — 0 (open, 9)
        // Frame 8
        (10, "Strike!"), // Alice — strike
        (7, ""),         // Bob — 7
        (3, "Spare!"),   // Bob — spare
        // Frame 9
        (9, ""),     // Alice — 9
        (0, "Miss"), // Alice — 0 (open, 9)
        (8, ""),     // Bob — 8
        (1, ""),     // Bob — 1 (open, 9)
        // Frame 10
        (10, "Strike!"), // Alice — strike (fill balls coming)
        (10, "Strike!"), // Alice — fill 1
        (10, "Strike!"), // Alice — fill 2
        (9, ""),         // Bob — 9
        (1, "Spare!"),   // Bob — spare (fill ball coming)
        (5, ""),         // Bob — fill
    ];

    let mut progress = Progress::AwaitingRoll(game);
    for &(pins, label) in rolls {
        let Progress::AwaitingRoll(g) = progress else {
            break;
        };
        let name = g.current_player().name().to_owned();
        let frame = g.current_frame_number();
        let ball = g.current_ball_number();

        let tag = if label.is_empty() {
            format!("{pins}")
        } else {
            format!("{pins} ({label})")
        };
        println!("  {name:<8} | frame {frame:>2}, ball {ball} | {tag}");

        progress = g.roll_count(pins).unwrap();
    }

    let Progress::Complete(game) = progress else {
        panic!("game should be done");
    };

    println!("\n--- Alice ---");
    println!("{}\n", game.scoreboard(0));

    println!("--- Bob ---");
    println!("{}\n", game.scoreboard(1));

    let winners = game.winners();
    if winners.len() == 1 {
        println!("{} wins!", winners[0].name());
    } else {
        let names: Vec<_> = winners.iter().map(|p| p.name()).collect();
        println!("Tie between: {}", names.join(", "));
    }
}
